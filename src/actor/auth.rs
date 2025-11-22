use crate::persistence::Query;
use uuid::Uuid;

use crate::messaging::{
    broker::MessageBroker,
    model::{Message, MessageBody},
};

pub struct AuthActorHandler {
    pub id: Uuid,
}

impl AuthActorHandler {
    pub fn new() -> Self {
        AuthActorHandler { id: Uuid::new_v4() }
    }

    pub async fn get_inventory_ids(broker: &MessageBroker) -> Vec<Uuid> {
        tracing::debug!("Requesting inventory IDs from persistence layer");
        let body = MessageBody::PersistenceQueryRequest(Query::GetInventoryIds);
        let response = broker
            .request(Message::new_request(body, Some("persistence".into())))
            .await;

        if let Ok(msg) = response {
            match msg {
                Some(m) => {
                    if let MessageBody::PersistenceQueryResponse(result) = m.body {
                        tracing::debug!("Received response from persistence layer: {:?}", result);
                        match result {
                            crate::persistence::QueryResponse::GetInventoryIds(ids) => {
                                return ids;
                            }
                            crate::persistence::QueryResponse::GetInventoryIdsFailed(reason) => {
                                tracing::warn!("Failed to get inventory IDs: {}", reason);
                            }
                            _ => {
                                tracing::warn!("Unexpected response to GetInventoryIds query");
                            }
                        }
                    }
                }
                None => {
                    tracing::warn!("No response received for GetInventoryIds query");
                }
            }
        } else {
            tracing::error!(
                "Error occurred while requesting GetInventoryIds: {:?}",
                response.err()
            );
        }

        vec![]
    }

    async fn authenticate(
        broker: &MessageBroker,
        username: &str,
        password: &str,
    ) -> Result<String, anyhow::Error> {
        let response = broker
            .request(Message {
                id: Uuid::new_v4(),
                body: MessageBody::PersistenceQueryRequest(Query::Auth {
                    username: username.into(),
                    password: password.into(),
                }),
                topic: Some("persistence".into()),
                is_request: true,
                timestamp: chrono::Utc::now().timestamp_millis() as u64,
            })
            .await?;

        tracing::debug!(
            response = format!("{response:?}"),
            "received response from persistence layer"
        );

        if let Some(msg) = response {
            if let MessageBody::PersistenceQueryResponse(result) = msg.body {
                match result {
                    crate::persistence::QueryResponse::AuthSuccess(token) => {
                        return Ok(token);
                    }
                    crate::persistence::QueryResponse::AuthFailed(reason) => {
                        tracing::warn!("Authentication failed: {}", reason);
                        return Err(anyhow::anyhow!("Authentication failed: {}", reason));
                    }
                    _ => {
                        tracing::warn!("Unexpected response to Auth query");
                    }
                }
            }
        } else {
            tracing::warn!("No response received for Auth query");
        }

        Err(anyhow::anyhow!("Authentication failed"))
    }

    pub async fn listen(&self, broker: MessageBroker) -> Result<(), anyhow::Error> {
        let (sub_id, mut rx) = match broker.subscribe("auth").await {
            Ok(id) => id,
            Err(e) => {
                eprintln!("Failed to subscribe to auth channel: {}", e);
                return Err(anyhow::anyhow!("Failed to subscribe to auth channel"));
            }
        };

        let subbroker = broker.clone();
        while let Some(msg) = rx.recv().await {
            tracing::info!("Received auth message: {:?}", msg);

            let reply_topic = msg.reply_topic();
            match msg.body {
                MessageBody::AuthenticationRequest { user, password } => {
                    let response = match Self::authenticate(&broker, &user, &password).await {
                        Ok(token) => Ok(token),
                        Err(e) => {
                            tracing::error!("Authentication error: {}", e);
                            Err("Authentication failed".into())
                        }
                    };

                    subbroker
                        .send(Message {
                            id: Uuid::new_v4(),
                            body: MessageBody::AuthenticationResponse(response),
                            topic: Some(reply_topic),
                            is_request: false,
                            timestamp: chrono::Utc::now().timestamp_millis() as u64,
                        })
                        .await?;
                }
                _ => tracing::warn!("Unexpected message body: {:?}", msg.body),
            }
        }

        let _ = broker.unsubscribe(sub_id).await;

        Ok(())
    }

    // pub async fn persist(&self) -> Result<(), anyhow::Error> {
    //     let db = self.db.read().await;
    //     let raw_data = serde_json::to_string(&*db)
    //         .map_err(|e| anyhow::anyhow!("Failed to serialize JSON: {}", e))?;

    //     fs::write("users.json", raw_data)
    //         .map_err(|e| anyhow::anyhow!("Failed to write to file: {}", e))?;
    //     Ok(())
    // }
}
