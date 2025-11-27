use std::sync::{Arc, Mutex};

use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::messaging::{
    broker::MessageBroker,
    model::{Message, MessageBody},
};

pub struct TickerActorHandler {
    pub id: Uuid,
    seq: Arc<Mutex<u64>>,
}

impl Default for TickerActorHandler {
    fn default() -> Self {
        Self::new()
    }
}

const TICKER_INTERVAL_MILLISECS: u64 = 1000;

impl TickerActorHandler {
    pub fn new() -> Self {
        TickerActorHandler {
            id: Uuid::new_v4(),
            seq: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn start(&self, broker: MessageBroker) -> Result<JoinHandle<()>, anyhow::Error> {
        let seq = Arc::clone(&self.seq);
        let broker = broker.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(
                TICKER_INTERVAL_MILLISECS,
            ));
            loop {
                interval.tick().await;

                // Increment and get seq
                let current_seq = {
                    let mut seq_guard = match seq.lock() {
                        Ok(guard) => guard,
                        Err(poisoned) => {
                            tracing::error!("mutex poisoned, recovering");
                            poisoned.into_inner()
                        }
                    };
                    *seq_guard = seq_guard.wrapping_add(1);
                    *seq_guard
                };

                let msg = Message::new(
                    MessageBody::Tick {
                        seq: current_seq,
                        timestamp: chrono::Utc::now(),
                    },
                    Some("ticks".into()),
                    false,
                );

                if let Err(e) = broker.send(msg).await {
                    tracing::error!("failed to publish ticker tick message: {}", e);
                } else {
                    tracing::trace!("sent ticker tick message");
                }
            }
        });

        Ok(handle)
    }
}
