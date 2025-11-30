use futures::stream::select_all;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::messaging::broker::MessageBroker;
use crate::messaging::model::{Message, MessageBody};
use crate::persistence::Query;

mod handler;

pub struct InventoryActorHandler {
    pub id: Uuid,
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
        tracing::debug!(
            "Inventory actor {} subscribed with id {}",
            self.id,
            tick_sub_id
        );

        let receivers = vec![inventory_rx, tick_rx];
        let mut streams = select_all(receivers.into_iter().map(ReceiverStream::new));

        let subbroker = broker.clone();
        while let Some(msg) = streams.next().await {
            tracing::trace!(?msg, "received inventory message");

            let reply_topic = msg.reply_topic();
            match msg.body {
                MessageBody::BuildRequest {
                    inventory_id,
                    blueprint_slug,
                } => {
                    tracing::info!(
                        "Received build request for inventory: {}, build: {}",
                        inventory_id,
                        blueprint_slug
                    );

                    let build_response =
                        handler::handle_build_request(&subbroker, inventory_id, blueprint_slug)
                            .await;

                    let reply = Message::new(build_response, Some(reply_topic.clone()), false);

                    subbroker.send(reply).await?;
                }
                MessageBody::Tick { seq, timestamp } => {
                    tracing::trace!(
                        seq,
                        timestamp = format!("{}", timestamp.to_rfc3339()),
                        actor_id = self.id.to_string(),
                        "Inventory actor received tick"
                    );

                    let response = subbroker
                        .request(Message::new_request(
                            MessageBody::PersistenceQueryRequest(Query::ProgressBuildings {
                                inventory_id: self.id,
                            }),
                            Some("persistence".into()),
                        ))
                        .await?;

                    tracing::trace!(
                        actor_id = self.id.to_string(),
                        "Inventory actor processed tick {}: persistence response: {:?}",
                        seq,
                        response
                    );
                }
                _ => tracing::warn!("Unexpected message body: {:?}", msg.body),
            }
        }

        let _ = broker.unsubscribe(sub_id).await;

        Ok(())
    }
}
