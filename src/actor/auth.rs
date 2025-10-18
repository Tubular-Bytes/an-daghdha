use crate::actor::model::User;
use std::{collections::HashMap, fs, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::messaging::{
    broker::MessageBroker,
    model::{Message, MessageBody},
};

pub struct AuthActorHandler {
    pub id: Uuid,
    db: Arc<RwLock<HashMap<Uuid, User>>>,
}

impl AuthActorHandler {
    pub fn load(source: String) -> Result<Self, anyhow::Error> {
        let raw_data = fs::read_to_string(source).expect("Failed to read source file");

        let users: HashMap<Uuid, User> = serde_json::from_str(&raw_data)
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?;

        tracing::debug!(users = ?users.len(), "loaded users");

        let db = Arc::new(RwLock::new(users));
        Ok(AuthActorHandler {
            id: Uuid::new_v4(),
            db,
        })
    }

    pub async fn get_inventory_ids(&self) -> Vec<Uuid> {
        let db = self.db.read().await;
        db.values().map(|u| u.id).collect()
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
                    let db = self.db.read().await;
                    let response = if let Some(user) = db
                        .values()
                        .find(|u| u.username == user && u.password == password)
                    {
                        let token = crate::auth::token::generate_token(user).map_err(|e| {
                            tracing::error!("Failed to generate token: {}", e);
                            anyhow::format_err!("Token generation failed")
                        })?;
                        tracing::info!("Authentication successful for user: {}", user.username);
                        Ok(token)
                    } else {
                        tracing::warn!("Authentication failed for user: {}", user);
                        Err("Authentication failed".into())
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

    pub async fn persist(&self) -> Result<(), anyhow::Error> {
        let db = self.db.read().await;
        let raw_data = serde_json::to_string(&*db)
            .map_err(|e| anyhow::anyhow!("Failed to serialize JSON: {}", e))?;

        fs::write("users.json", raw_data)
            .map_err(|e| anyhow::anyhow!("Failed to write to file: {}", e))?;
        Ok(())
    }
}
