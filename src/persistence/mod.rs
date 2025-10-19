use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::messaging::{
    broker::MessageBroker,
    model::{Message, MessageBody},
};

const TOPIC: &str = "persistence";

#[derive(Debug)]
pub struct PersistenceHandler {}

impl PersistenceHandler {
    pub async fn listen(broker: &MessageBroker) -> Result<JoinHandle<()>, anyhow::Error> {
        let broker = broker.clone();
        let handle = tokio::spawn(async move {
            let (sub_id, mut rx) = match broker.subscribe(TOPIC).await {
                Ok(sub) => sub,
                Err(e) => {
                    tracing::error!(
                        error = e.to_string(),
                        "Failed to subscribe to persistence channel"
                    );
                    return;
                }
            };

            tracing::debug!(
                topic = TOPIC,
                subscription_id = sub_id.to_string(),
                "subscribed to broker topic",
            );

            while let Some(msg) = rx.recv().await {
                tracing::info!("Persistence handler received message: {:?}", msg);

                if msg.is_request {
                    let reply = Message {
                        id: Uuid::new_v4(),
                        body: MessageBody::PersistenceQueryResponse(vec![]),
                        topic: Some(msg.reply_topic()),
                        is_request: false,
                        timestamp: chrono::Utc::now().timestamp_millis() as u64,
                    };

                    if let Err(e) = broker.send(reply).await {
                        tracing::error!(
                            error = e.to_string(),
                            "Failed to send persistence query response"
                        );
                    }
                }
                // match msg.body {
                //     _ => {
                //         tracing::warn!(
                //             "Persistence handler received unexpected message body: {:?}",
                //             msg.body
                //         );
                //     }
                // }
            }
        });

        Ok(handle)
    }
}
