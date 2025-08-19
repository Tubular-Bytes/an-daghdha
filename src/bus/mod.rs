use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard};

use crate::bus::handlers::MessageHandler;
use crate::bus::model::Message;

pub mod handlers;
pub mod model;

#[derive(Clone, PartialEq)]
pub enum Status {
    Unstarted,
    Running,
    Stopped,
}

pub struct Broker {
    tx: tokio::sync::broadcast::Sender<Message>,
    _rx: tokio::sync::broadcast::Receiver<Message>,

    status: Arc<RwLock<Status>>,
}

impl Broker {
    pub fn new() -> (Self, tokio::sync::broadcast::Sender<Message>) {
        let (tx, _rx) = tokio::sync::broadcast::channel(100);

        (
            Self {
                tx: tx.clone(),
                _rx,
                status: Arc::new(RwLock::new(Status::Unstarted)),
            },
            tx.clone(),
        )
    }

    /// Helper method to send a message and wait for a response
    pub async fn send_authentication_request(
        tx: &tokio::sync::broadcast::Sender<Message>,
        username: String,
        password: String,
    ) -> Result<model::AuthenticationResponse, Box<dyn std::error::Error + Send + Sync>> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();

        let msg = Message::Authentication {
            username,
            password,
            reply: std::sync::Arc::new(reply_tx),
        };

        tx.send(msg)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        reply_rx
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    /// Helper method to send an authorization request
    pub async fn send_authorization_request(
        tx: &tokio::sync::broadcast::Sender<Message>,
        token: String,
    ) -> Result<model::AuthorizationResponse, Box<dyn std::error::Error + Send + Sync>> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();

        let msg = Message::Authorization {
            token,
            reply: std::sync::Arc::new(reply_tx),
        };

        tx.send(msg)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        reply_rx
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    /// Helper method to send a get request
    pub async fn send_get_request(
        tx: &tokio::sync::broadcast::Sender<Message>,
        key: String,
    ) -> Result<model::GetResponse, Box<dyn std::error::Error + Send + Sync>> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();

        let msg = Message::Get {
            key,
            reply: Some(std::sync::Arc::new(reply_tx)),
        };

        tx.send(msg)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        reply_rx
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    /// Helper method to send a set request
    pub async fn send_set_request(
        tx: &tokio::sync::broadcast::Sender<Message>,
        key: String,
        value: String,
    ) -> Result<model::SetResponse, Box<dyn std::error::Error + Send + Sync>> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();

        let msg = Message::Set {
            key,
            value,
            reply: Some(std::sync::Arc::new(reply_tx)),
        };

        tx.send(msg)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        reply_rx
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    /// Helper method to send a delete request
    pub async fn send_delete_request(
        tx: &tokio::sync::broadcast::Sender<Message>,
        key: String,
    ) -> Result<model::DeleteResponse, Box<dyn std::error::Error + Send + Sync>> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();

        let msg = Message::Delete {
            key,
            reply: Some(std::sync::Arc::new(reply_tx)),
        };

        tx.send(msg)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        reply_rx
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    pub fn status(&self) -> Result<Status, PoisonError<RwLockReadGuard<Status>>> {
        let status = self.status.read()?.clone();
        Ok(status)
    }

    pub async fn start(&mut self) {
        let _ttx = self.tx.clone();
        let mut rx = self.tx.subscribe();
        let status = self.status.clone();
        tokio::spawn(async move {
            {
                let mut status_guard = status.write().unwrap();
                *status_guard = Status::Running;
            }
            tracing::info!("starting broker loop");

            loop {
                match rx.recv().await {
                    Ok(msg) => {
                        tracing::info!("received message: {:?}", std::mem::discriminant(&msg));

                        match msg {
                            Message::Shutdown => {
                                tracing::info!("received shutdown message");
                                break;
                            }
                            _ => {
                                MessageHandler::dispatch(msg);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(error=?e, "failed to receive message");
                    }
                }
            }

            tracing::info!("broker loop finished");
            {
                let mut status_guard = status.write().unwrap();
                *status_guard = Status::Stopped;
            }
        });
    }
}
