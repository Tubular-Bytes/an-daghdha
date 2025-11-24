use uuid::Uuid;

use crate::messaging::broker::MessageBroker;
use crate::messaging::model::{Message, MessageBody};

pub struct InventoryActorHandler {
    pub id: Uuid,
    // todo implement actual database handling
    // db: Arc<RwLock<HashMap<Uuid, User>>>,
}

impl InventoryActorHandler {
    pub async fn listen(&self, broker: MessageBroker) -> Result<(), anyhow::Error> {
        let topic = format!("in:inventory:{}", self.id);
        let (sub_id, mut rx) = match broker.subscribe(topic.as_str()).await {
            Ok(id) => {
                tracing::debug!(
                    actor_id = self.id.to_string(),
                    topic,
                    sub_id = id.0.to_string(),
                    "inventory actor subscribed to incoming messages"
                );
                id
            }
            Err(e) => {
                eprintln!("Failed to subscribe to inventory channel: {}", e);
                return Err(anyhow::anyhow!("Failed to subscribe to inventory channel"));
            }
        };

        tracing::debug!("Inventory actor {} subscribed with id {}", self.id, sub_id);

        let subbroker = broker.clone();
        while let Some(msg) = rx.recv().await {
            tracing::info!("Received inventory message: {:?}", msg);

            let reply_topic = msg.reply_topic();
            match msg.body {
                MessageBody::BuildRequest {
                    inventory_id,
                    blueprint_id,
                } => {
                    tracing::info!(
                        "Received build request for inventory: {}, build: {}",
                        inventory_id,
                        blueprint_id
                    );

                    subbroker
                        .send(Message {
                            id: Uuid::new_v4(),
                            body: MessageBody::BuildResponse(Ok(format!(
                                "Build started for blueprint: {}",
                                blueprint_id
                            ))),
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
}
