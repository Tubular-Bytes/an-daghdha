use axum::extract::ws::{Message, WebSocket};
use futures_util::SinkExt;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

use futures::stream::select_all;
use futures::StreamExt;
use tokio_stream::wrappers::ReceiverStream;

use crate::messaging::{broker::MessageBroker, model::Message as BusMessage, model::MessageBody};

type PeerMap = Arc<Mutex<HashMap<Uuid, WebSocket>>>;

lazy_static::lazy_static! {
    static ref EMPTY_MESSAGE: BusMessage = BusMessage::new(MessageBody::Empty, None, false);
}

#[derive(Clone)]
pub struct Bouncer {
    _peers: PeerMap,
    broker: MessageBroker,
}

impl Bouncer {
    pub fn new(broker: &MessageBroker) -> Self {
        Bouncer {
            _peers: Arc::new(Mutex::new(HashMap::new())),
            broker: broker.clone(),
        }
    }

    pub async fn handle_connection(self, user_id: Uuid, ws: WebSocket) {
        tracing::info!("New WebSocket connection from {user_id}");

        let (mut sink, mut stream) = ws.split();

        sink.send(Message::Ping("Ping".into()))
            .await
            .unwrap_or_else(|e| {
                tracing::error!("Failed to send ping message: {}", e);
            });

        let (_, global_rx) = match self.broker.subscribe("global").await {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("Failed to subscribe to global topic: {}", e);
                return;
            }
        };
        let (_, inventory_rx) = match self.broker.subscribe(&format!("inventory:{user_id}")).await {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("Failed to subscribe to inventory topic: {}", e);
                return;
            }
        };

        let receivers = vec![global_rx, inventory_rx];
        let mut merged_rx = select_all(receivers.into_iter().map(ReceiverStream::new));

        let outgoing = tokio::spawn(async move {
            let sink = &mut sink;
            while let Some(msg) = merged_rx.next().await {
                if let Ok(text) = serde_json::to_string(&msg) {
                    sink.send(Message::Text(text.into()))
                        .await
                        .unwrap_or_else(|e| {
                            tracing::error!("Failed to send message: {}", e);
                        });
                }
            }
        });

        while let Some(Ok(message)) = stream.next().await {
            if let Message::Text(name) = message {
                tracing::info!("Received message: {}", name);
                self.broker
                    .send(BusMessage {
                        id: Uuid::new_v4(),
                        body: MessageBody::Empty,
                        topic: Some("global".to_string()),
                        is_request: false,
                        timestamp: 0,
                    })
                    .await
                    .unwrap_or_else(|e| {
                        tracing::error!("Failed to send message to broker: {}", e);
                    });
            } else if let Message::Close(_) = message {
                tracing::info!("WebSocket connection closed");
                break;
            }
        }

        let _ = outgoing.await;
    }

    // pub async fn _handle_connection(self, addr: SocketAddr, stream: TcpStream) {
    //     tracing::debug!("handling incoming connection: {:?}", addr);

    //     let mut used_headers = http::HeaderMap::new();

    //     let callback = |req: &Request, response: Response| {
    //         let headers = req.headers();
    //         tracing::debug!("got auth header: {:?}", headers.get("Authorization"));
    //         if let Some(auth_header) = headers.get("Authorization") {
    //             used_headers.insert("Authorization", auth_header.clone());
    //         }

    //         Ok(response)
    //     };

    //     let ws_stream = match tokio_tungstenite::accept_hdr_async(stream, callback).await {
    //         Ok(stream) => stream,
    //         Err(err) => {
    //             tracing::error!("Error during the websocket handshake: {}", err);
    //             return;
    //         }
    //     };
    //     tracing::debug!("webSocket connection established: {}", addr);

    //     let (mut write, mut read) = ws_stream.split();

    //     while let Some(msg) = read.next().await {
    //         match msg {
    //             Ok(msg) => {
    //                 if !msg.is_text() {
    //                     tracing::warn!("Received non-text message: {:?}", msg);
    //                     continue;
    //                 }
    //                 let response = self.handle_message(&mut used_headers, msg.clone()).await;
    //                 match response {
    //                     Ok(resp) => {
    //                         tracing::debug!("Sending response: {:?}", resp);
    //                         write
    //                             .send(Message::text(format!("Response: {:?}", resp)))
    //                             .await
    //                             .unwrap_or_else(|e| {
    //                                 tracing::error!("Failed to send response: {}", e);
    //                             });
    //                     }
    //                     Err(e) => {
    //                         tracing::error!("Error handling message: {}", e);
    //                         let msg = Message::text(format!("Error: {}", e));
    //                         write.send(msg).await.unwrap_or_else(|e| {
    //                             tracing::error!("Failed to send error message: {}", e);
    //                         });
    //                     }
    //                 }
    //             }
    //             Err(e) => {
    //                 tracing::error!("Error receiving message: {}", e);
    //                 break;
    //             }
    //         }
    //     }

    //     tracing::debug!("{} disconnected", addr);
    //     self.peers.lock().unwrap().remove(&addr);
    // }

    // async fn handle_message(
    //     &self,
    //     headers: &mut http::HeaderMap,
    //     message: Message,
    // ) -> Result<BusMessage, anyhow::Error> {
    //     tracing::debug!("proxied headers: {headers:?}");
    //     let msg: Value = serde_json::from_str(message.to_text()?)
    //         .map_err(|e| anyhow::anyhow!("Failed to parse message: {}", e))?;
    //     tracing::debug!("parsed message: {msg:?}");

    //     let mut message_body = MessageBody::from_value(&msg)?;

    //     match &mut message_body {
    //         MessageBody::AuthenticationRequest { .. } => {
    //             tracing::debug!("forwarding auth request to auth actor");
    //             let message = BusMessage::new(message_body, topic("auth"), true);

    //             match self.broker.request(message).await {
    //                 Ok(response) => {
    //                     tracing::debug!("Received response from auth actor: {response:?}");
    //                     headers.insert("X-Logged-In", "true".parse().unwrap());
    //                     Ok(response.unwrap())
    //                 }
    //                 Err(e) => {
    //                     tracing::error!("Error sending message to bus: {}", e);
    //                     Err(anyhow::anyhow!("Error sending message to bus: {}", e))
    //                 }
    //             }
    //         }
    //         MessageBody::BuildRequest {
    //             inventory_id,
    //             blueprint_id,
    //         } => {
    //             tracing::debug!("forwarding build request to build actor");
    //             let mutated_body = MessageBody::BuildRequest {
    //                 inventory_id: *inventory_id,
    //                 blueprint_id: blueprint_id.clone(),
    //             };
    //             let message = BusMessage::new(
    //                 mutated_body,
    //                 topic(format!("inventory:{inventory_id}").as_str()),
    //                 false,
    //             );

    //             let _ = self.broker.send(message).await;
    //             Ok(EMPTY_MESSAGE.clone())

    //             // match self.broker.request(message).await {
    //             //     Ok(response) => {
    //             //         tracing::debug!("Received response from build actor: {response:?}");
    //             //         Ok(response.unwrap())
    //             //     }
    //             //     Err(e) => {
    //             //         tracing::error!("Error sending message to bus: {}", e);
    //             //         Err(anyhow::anyhow!("Error sending message to bus: {}", e))
    //             //     }
    //             // }
    //         }

    //         _ => {
    //             tracing::warn!("unexpected message body: {:?}", message_body);
    //             Err(anyhow::anyhow!("unexpected message body"))
    //         }
    //     }
    // }
}

pub fn topic(name: &str) -> Option<String> {
    match name {
        "" | "none" => None,
        _ => Some(format!("topic:{}", name)),
    }
}
