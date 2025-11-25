use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};

use super::model::{Message, MessageBody, Status, Subscription};

pub struct MessageHandler {
    status: Arc<RwLock<Status>>,
    inbox: mpsc::Receiver<Message>,
    subscriptions: Arc<RwLock<Vec<Subscription>>>,
}

impl MessageHandler {
    pub fn new(
        inbox: mpsc::Receiver<Message>,
        subscriptions: Arc<RwLock<Vec<Subscription>>>,
        status: Arc<RwLock<Status>>,
    ) -> Self {
        MessageHandler {
            status,
            inbox,
            subscriptions,
        }
    }

    pub async fn start(&mut self) {
        {
            let mut status_guard = self.status.write().await;
            *status_guard = Status::Running;
        }
        while let Some(message) = self.inbox.recv().await {
            if std::mem::discriminant(&message.body) == std::mem::discriminant(&MessageBody::Stop) {
                tracing::info!("Received stop message, shutting down");
                {
                    let mut status_guard = self.status.write().await;
                    *status_guard = Status::Stopping;
                }
                break;
            }

            self.handle_message(message).await;
        }
        {
            let mut status_guard = self.status.write().await;
            *status_guard = Status::Stopped;
        }
    }

    pub async fn handle_message(&mut self, message: Message) {
        tracing::trace!(message=format!("{:?}", message), "handling message");

        // Forward message to all matching subscriptions
        if let Some(topic) = &message.topic {
            let subscriptions = self.subscriptions.read().await;
            for subscription in subscriptions.iter() {
                tracing::trace!(
                    topic,
                    subscription = subscription.pattern.as_str(),
                    "checking for subscriptions"
                );

                if subscription.pattern.is_match(topic) {
                    if let Err(e) = subscription.tx.send(message.clone()).await {
                        tracing::warn!("Failed to send message to subscriber: {}", e);
                        continue;
                    }

                    tracing::debug!(
                        topic = topic.as_str(),
                        subscription_id = subscription.id.to_string(),
                        "message forwarded to subscriber"
                    );
                }
            }
        }
    }
}
