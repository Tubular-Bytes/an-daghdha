use std::sync::Arc;

use regex::Regex;
use tokio::sync::{broadcast, mpsc, oneshot, RwLock};
use uuid::Uuid;

pub enum Status {
    Unstarted,
    Running,
    Stopping,
    Stopped,
}

#[derive(Debug, Clone)]
pub enum MessageBody {
    Example { foo: String },
    ExampleResponse { foo: String },
    Stop,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: Uuid,
    pub body: MessageBody,
    pub topic: Option<String>,
    pub is_request: bool,
    pub timestamp: u64,
}

impl Message {
    pub fn reply_topic(&self) -> String {
        format!("reply-{}", self.id)
    }
}

pub struct Topic {
    pub name: String,
    pub tx: broadcast::Sender<Message>,
    pub _rx: broadcast::Receiver<Message>,
}

impl Topic {
    pub fn new(name: String) -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Topic { name, tx, _rx }
    }
}

pub struct Subscription {
    pub id: Uuid,
    pub pattern: Regex,
    pub tx: mpsc::Sender<Message>,
}

impl Subscription {
    pub fn new(pattern: &str) -> Result<(Self, mpsc::Receiver<Message>), regex::Error> {
        let regex = Regex::new(pattern)?;
        let (tx, rx) = mpsc::channel(100);
        Ok((Self { id: Uuid::new_v4(), pattern: regex, tx }, rx))
    }
}

#[derive(Clone)]
pub struct MessageBroker {
    tx: mpsc::Sender<Message>,
    subscriptions: Arc<RwLock<Vec<Subscription>>>,
}

impl MessageBroker {
    pub fn new() -> (Self, MessageHandler) {
        let (tx, rx) = mpsc::channel(100);
        let subscriptions = Arc::new(RwLock::new(Vec::new()));
        
        let broker = MessageBroker {
            tx: tx.clone(),
            subscriptions: subscriptions.clone(),
        };
        
        let handler = MessageHandler::new(rx, subscriptions);
        
        (broker, handler)
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

pub struct MessageHandler {
    status: Arc<RwLock<Status>>,
    inbox: mpsc::Receiver<Message>,
    subscriptions: Arc<RwLock<Vec<Subscription>>>,
}

impl MessageHandler {
    fn new(inbox: mpsc::Receiver<Message>, subscriptions: Arc<RwLock<Vec<Subscription>>>) -> Self {
        MessageHandler {
            status: Arc::new(RwLock::new(Status::Unstarted)),
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
            if std::mem::discriminant(&message.body)
                == std::mem::discriminant(&MessageBody::Stop)
            {
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
        tracing::info!("Handling message: {:?}", message);
        
        // Forward message to all matching subscriptions
        if let Some(topic) = &message.topic {
            let subscriptions = self.subscriptions.read().await;
            for subscription in subscriptions.iter() {
                if subscription.pattern.is_match(topic) {
                    if let Err(e) = subscription.tx.send(message.clone()).await {
                        tracing::warn!("Failed to send message to subscriber: {}", e);
                    }
                }
            }
        }
    }
}
