use futures::stream::select_all;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::{StreamExt};
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
        let inventory_topic = format!("in:inventory:{}", self.id);
        let (sub_id, inventory_rx) = match broker.subscribe(inventory_topic.as_str()).await {
            Ok(id) => {
                tracing::debug!(
                    actor_id = self.id.to_string(),
                    inventory_topic,
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

        let tick_topic = "ticks";
        let (tick_sub_id, tick_rx) = match broker.subscribe(tick_topic).await {
            Ok(id) => {
                tracing::debug!(
                    actor_id = self.id.to_string(),
                    topic = tick_topic,
                    sub_id = id.0.to_string(),
                    "inventory actor subscribed to tick messages"
                );
                id
            }
            Err(e) => {
                eprintln!("Failed to subscribe to tick channel: {}", e);
                return Err(anyhow::anyhow!("Failed to subscribe to tick channel"));
            }
        };
        tracing::debug!("Inventory actor {} subscribed with id {}", self.id, tick_sub_id);

        let receivers = vec![inventory_rx, tick_rx];
        let mut streams = select_all(receivers.into_iter().map(ReceiverStream::new));

        let subbroker = broker.clone();
        while let Some(msg) = streams.next().await {
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
                MessageBody::Tick { seq, timestamp } => {
                    tracing::info!(seq, timestamp=format!("{}", timestamp.to_rfc3339()), actor_id=self.id.to_string(), "Inventory actor received tick");
                    // Handle tick event, e.g., update inventory status
                }
                _ => tracing::warn!("Unexpected message body: {:?}", msg.body),
            }
        }

        let _ = broker.unsubscribe(sub_id).await;

        Ok(())
    }
}
