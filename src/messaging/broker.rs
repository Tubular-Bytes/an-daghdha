use std::sync::Arc;

use tokio::sync::{mpsc, oneshot, RwLock};
use uuid::Uuid;

use super::{
    model::{Message, Subscription, Status, MessageBody},
    handler::MessageHandler,
};

#[derive(Clone)]
pub struct MessageBroker {
    tx: mpsc::Sender<Message>,
    subscriptions: Arc<RwLock<Vec<Subscription>>>,
    status: Arc<RwLock<Status>>,
}

impl MessageBroker {
    pub fn new() -> (Self, MessageHandler) {
        let (tx, rx) = mpsc::channel(100);
        let subscriptions = Arc::new(RwLock::new(Vec::new()));
        let status = Arc::new(RwLock::new(Status::Unstarted));
        
        let broker = MessageBroker {
            tx: tx.clone(),
            subscriptions: subscriptions.clone(),
            status: status.clone(),
        };
        
        let handler = MessageHandler::new(rx, subscriptions, status.clone());
        
        (broker, handler)
    }

    pub async fn status(&self) -> Status {
        let status = self.status.read().await;
        status.clone()
    }
    
    pub async fn send(&self, message: Message) -> Result<(), mpsc::error::SendError<Message>> {
        self.tx.send(message).await
    }

    pub async fn request(&self, message: Message) -> Result<Option<Message>, anyhow::Error> {
        // if the message is not a request we should just send a fire and forget and return None as the result
        if !message.is_request {
            self.send(message).await?;
            return Ok(None);
        }

        let reply_topic = message.reply_topic();
        let (result_tx, result_rx) = oneshot::channel();
        
        let broker = self.clone();
        
        self.tx.send(message).await?;
        tracing::debug!("message sent, spawning reply handler for topic: {}", reply_topic);
        
        tokio::spawn(async move {
            let reply_result = async {
                let (reply_id, mut subscriber_reply) = broker.subscribe(&reply_topic).await
                    .map_err(|e| anyhow::format_err!("Failed to subscribe: {}", e))?;
                
                tracing::debug!("subscribed to topic: {}", reply_topic);
                
                let reply = tokio::time::timeout(
                    std::time::Duration::from_secs(30), // 30 second timeout
                    subscriber_reply.recv()
                ).await;
                
                if let Err(e) = broker.unsubscribe(reply_id).await {
                    tracing::warn!("Failed to unsubscribe: {}", e);
                }
                
                match reply {
                    Ok(Some(message)) => {
                        tracing::debug!("received reply: {:?}", message);
                        Ok(Some(message))
                    }
                    Ok(None) => {
                        tracing::debug!("reply channel closed");
                        Ok(None)
                    }
                    Err(_) => {
                        tracing::warn!("timeout waiting for reply on topic: {}", reply_topic);
                        Err(anyhow::format_err!("Timeout waiting for reply"))
                    }
                }
            }.await;
            
            let _ = result_tx.send(reply_result);
        });
        
        result_rx.await
            .map_err(|e| anyhow::format_err!("Reply handler task failed: {}", e))?
    }
    
    pub async fn subscribe(&self, pattern: &str) -> Result<(Uuid, mpsc::Receiver<Message>), regex::Error> {
        let (subscription, rx) = Subscription::new(pattern)?;
        let subscription_id = subscription.id;
        self.subscriptions.write().await.push(subscription);
        Ok((subscription_id, rx))
    }
    
    pub async fn unsubscribe(&self, subscription_id: Uuid) -> Result<(), String> {
        tracing::info!("Unsubscribing from subscription ID: {}", subscription_id);
        let mut subscriptions = self.subscriptions.write().await;
        if let Some(pos) = subscriptions.iter().position(|s| s.id == subscription_id) {
            subscriptions.remove(pos);
            Ok(())
        } else {
            Err("Subscription not found".to_string())
        }
    }
}
