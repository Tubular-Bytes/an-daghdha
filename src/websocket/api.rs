use axum::extract::ws::{Message, WebSocket};
use futures_util::SinkExt;
use tokio::sync::mpsc;
use uuid::Uuid;

use futures::stream::select_all;
use futures::StreamExt;
use tokio_stream::wrappers::ReceiverStream;

use crate::{
    messaging::{
        broker::MessageBroker,
        model::{Message as BusMessage, MessageBody},
    },
    websocket::model::{RtcRequest, RtcRequestBody, RtcResponse},
};

lazy_static::lazy_static! {
    static ref EMPTY_MESSAGE: BusMessage = BusMessage::new(MessageBody::Empty, None, false);
}

#[derive(Clone)]
pub struct Bouncer {
    broker: MessageBroker,
}

impl Bouncer {
    pub fn new(broker: &MessageBroker) -> Self {
        Bouncer {
            broker: broker.clone(),
        }
    }

    async fn get_inventory_id(&self, user_id: Uuid) -> Result<Uuid, anyhow::Error> {
        let response = match self
            .broker
            .request(BusMessage::new_request(
                MessageBody::PersistenceQueryRequest(
                    crate::persistence::Query::GetInventoryForUser { user_id },
                ),
                topic("persistence"),
            ))
            .await?
        {
            Some(msg) => msg,
            None => {
                return Err(anyhow::anyhow!(
                    "No response received for inventory ID request"
                ))
            }
        };

        if let MessageBody::PersistenceQueryResponse(query_response) = response.body {
            match query_response {
                crate::persistence::QueryResponse::GetInventoryIdForUser(inventory_id) => {
                    Ok(inventory_id)
                }
                crate::persistence::QueryResponse::GetInventoryIdForUserFailed(err_msg) => {
                    Err(anyhow::anyhow!(err_msg))
                }
                _ => Err(anyhow::anyhow!(
                    "Unexpected query response for GetInventoryForUser"
                )),
            }
        } else {
            Err(anyhow::anyhow!(
                "Unexpected message body in response for GetInventoryForUser"
            ))
        }
    }

    pub async fn handle_connection(self, user_id: Uuid, ws: WebSocket) {
        tracing::info!(user_id = user_id.to_string(), "new WebSocket connection");

        let (mut sink, mut stream) = ws.split();

        let inventory_id = match self.get_inventory_id(user_id).await {
            Ok(id) => id,
            Err(e) => {
                tracing::error!(
                    user_id = user_id.to_string(),
                    error = e.to_string(),
                    "Failed to get inventory ID for user"
                );
                sink.send(Message::Text(
                    format!("failed to get inventory ID: {}", e).into(),
                ))
                .await
                .unwrap_or_else(|e| {
                    tracing::error!("Failed to send error message: {}", e);
                });
                return;
            }
        };

        sink.send(Message::Ping("Ping".into()))
            .await
            .unwrap_or_else(|e| {
                tracing::error!("Failed to send ping message: {}", e);
            });

        let (internal_sink_tx, mut internal_sink_rx) = mpsc::channel::<RtcResponse>(100);

        let (_, global_rx) = match self.broker.subscribe("global").await {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("Failed to subscribe to global topic: {}", e);
                sink.send(Message::Text(
                    format!("failed to subscribe to global topic: {}", e).into(),
                ))
                .await
                .unwrap_or_else(|e| {
                    tracing::error!("Failed to send error message: {}", e);
                });
                return;
            }
        };
        let (_, account_rx) = match self
            .broker
            .subscribe(&format!("out:account:{user_id}"))
            .await
        {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("Failed to subscribe to account topic: {}", e);
                sink.send(Message::Text(
                    format!("failed to subscribe to account topic: {}", e).into(),
                ))
                .await
                .unwrap_or_else(|e| {
                    tracing::error!("Failed to send error message: {}", e);
                });
                return;
            }
        };
        let (_, inventory_rx) = match self
            .broker
            .subscribe(&format!("out:inventory:{inventory_id}"))
            .await
        {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("Failed to subscribe to inventory topic: {}", e);
                sink.send(Message::Text(
                    format!("failed to subscribe to inventory topic: {}", e).into(),
                ))
                .await
                .unwrap_or_else(|e| {
                    tracing::error!("Failed to send error message: {}", e);
                });
                return;
            }
        };

        let combined_tx = internal_sink_tx.clone();
        let combined_stream = select_all(vec![
            ReceiverStream::new(global_rx),
            ReceiverStream::new(account_rx),
            ReceiverStream::new(inventory_rx),
        ]);
        // transform incoming BusMessages into RtcResponses and send them to internal_sink_tx
        let combiner = tokio::spawn(async move {
            let mut stream = combined_stream;

            while let Some(msg) = stream.next().await {
                let response = match RtcResponse::from_message(msg) {
                    Ok(resp) => resp,
                    Err(e) => {
                        tracing::error!("Failed to convert BusMessage to RtcResponse: {}", e);
                        RtcResponse {
                            id: Uuid::new_v4(),
                            success: false,
                            message: Some(format!("Internal server error: {}", e)),
                        }
                    }
                };

                combined_tx.send(response).await.unwrap_or_else(|e| {
                    tracing::error!("Failed to send message to internal sink: {}", e);
                });
            }
        });

        let outgoing = tokio::spawn(async move {
            let sink = &mut sink;

            while let Some(msg) = internal_sink_rx.recv().await {
                if let Ok(text) = serde_json::to_string(&msg) {
                    sink.send(Message::Text(text.into()))
                        .await
                        .unwrap_or_else(|e| {
                            tracing::error!("Failed to send message: {}", e);
                        });
                }
            }
        });

        let internal_tx = internal_sink_tx.clone();
        while let Some(Ok(message)) = stream.next().await {
            if let Message::Text(msg) = message {
                let request = match serde_json::from_str::<RtcRequest>(&msg) {
                    Ok(req) => req,
                    Err(e) => {
                        tracing::error!("Failed to parse incoming message: {}", e);
                        internal_tx
                            .send(RtcResponse {
                                id: Uuid::new_v4(),
                                success: false,
                                message: Some(format!("Invalid request format: {}", e)),
                            })
                            .await
                            .unwrap_or_else(|e| {
                                tracing::error!("Failed to send error message: {}", e);
                            });
                        continue;
                    }
                };

                tracing::trace!(
                    user_id = user_id.to_string(),
                    inventory_id = inventory_id.to_string(),
                    request = format!("{:?}", request),
                    "received RTC request"
                );

                let message = match request.body {
                    RtcRequestBody::Build { blueprint } => BusMessage::new(
                        MessageBody::BuildRequest {
                            inventory_id,
                            blueprint_slug: blueprint,
                        },
                        Some(format!("in:inventory:{}", inventory_id)),
                        true,
                    ),
                };

                match self.broker.request(message).await {
                    Ok(result) => {
                        tracing::info!(
                            user_id = user_id.to_string(),
                            "forwarded RTC request to message broker"
                        );

                        if let Some(reply) = result {
                            match RtcResponse::from_message(reply) {
                                Ok(response) => {
                                    internal_tx.send(response).await.unwrap_or_else(|e| {
                                        tracing::error!("Failed to send response message: {}", e);
                                    });
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Failed to convert reply to RtcResponse: {}",
                                        e
                                    );
                                    internal_tx
                                        .send(RtcResponse {
                                            id: Uuid::new_v4(),
                                            success: false,
                                            message: Some(format!("Internal server error: {}", e)),
                                        })
                                        .await
                                        .unwrap_or_else(|e| {
                                            tracing::error!("Failed to send error message: {}", e);
                                        });
                                }
                            }
                        } else {
                            tracing::debug!("No reply received for RTC request");
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to send message to broker: {}", e);
                        internal_tx
                            .send(RtcResponse {
                                id: Uuid::new_v4(),
                                success: false,
                                message: Some(format!("Invalid request format: {}", e)),
                            })
                            .await
                            .unwrap_or_else(|e| {
                                tracing::error!("Failed to send error message: {}", e);
                            });
                    }
                }
            } else if let Message::Close(_) = message {
                tracing::info!("WebSocket connection closed");
                break;
            }
        }

        let _ = outgoing.await;
        let _ = combiner.await;
    }
}

pub fn topic(name: &str) -> Option<String> {
    match name {
        "" | "none" => None,
        _ => Some(format!("topic:{}", name)),
    }
}
