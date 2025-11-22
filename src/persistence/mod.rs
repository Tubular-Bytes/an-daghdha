use std::sync::{Arc, RwLock};

use diesel::{r2d2::ConnectionManager, PgConnection};
use r2d2::Pool;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use uuid::Uuid;

mod user_repository;

use crate::messaging::{
    broker::MessageBroker,
    model::{Message, MessageBody},
};

const TOPIC: &str = "persistence";

#[derive(Debug, Clone, PartialEq)]
pub enum HandlerStatus {
    Initialized,
    Listening,
}

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Clone)]
pub struct PersistenceHandler {
    pub status: Arc<RwLock<HandlerStatus>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Query {
    Auth { username: String, password: String },
    GetInventoryIds,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum QueryResponse {
    // Placeholder for actual response types
    AuthSuccess(String),
    AuthFailed(String),

    GetInventoryIds(Vec<Uuid>),
    GetInventoryIdsFailed(String),
}

impl PersistenceHandler {
    pub fn new() -> Self {
        Self{
            status: Arc::new(RwLock::new(HandlerStatus::Initialized)),
        }
    }

    pub async fn listen(
        &mut self,
        broker: &MessageBroker,
        pool: &DbPool,
    ) -> Result<JoinHandle<()>, anyhow::Error> {
        let broker = broker.clone();
        let pool = pool.clone();
        let status = self.status.clone();
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

            match status.write() {
                Ok(mut s) => {
                    *s = HandlerStatus::Listening;
                }
                Err(e) => {
                    tracing::error!(
                        error = e.to_string(),
                        "Failed to update persistence handler status"
                    );
                }
            }

            while let Some(msg) = rx.recv().await {
                tracing::info!("Persistence handler received message: {:?}", msg);
                let reply_topic = msg.reply_topic();
                let conn = &mut pool.get().expect("Failed to get DB connection from pool");
                match msg.body {
                    MessageBody::PersistenceQueryRequest(query) => {
                        tracing::info!("Querying persistence data with query: {:?}", query);

                        match query {
                            Query::GetInventoryIds => {
                                tracing::debug!("received GetInventoryIds query");
                                let reply_body =
                                    match user_repository::get_inventory_ids(conn).await {
                                        Ok(ids) => MessageBody::PersistenceQueryResponse(
                                            QueryResponse::GetInventoryIds(ids),
                                        ),
                                        Err(e) => MessageBody::PersistenceQueryResponse(
                                            QueryResponse::GetInventoryIdsFailed(e.to_string()),
                                        ),
                                    };

                                let message = Message {
                                    id: Uuid::new_v4(),
                                    body: reply_body,
                                    topic: Some(reply_topic),
                                    is_request: false,
                                    timestamp: chrono::Utc::now().timestamp_millis() as u64,
                                };

                                if let Err(e) = broker.send(message).await {
                                    tracing::error!(
                                        error = e.to_string(),
                                        "Failed to send persistence query response"
                                    );
                                }
                            }
                            Query::Auth { username, password } => {
                                let id = user_repository::authenticate(conn, &username, &password)
                                    .await
                                    .unwrap_or(None);

                                let reply = match id {
                                    None => Message {
                                        id: Uuid::new_v4(),
                                        body: MessageBody::PersistenceQueryResponse(
                                            QueryResponse::AuthFailed(
                                                "Authentication failed".into(),
                                            ),
                                        ),
                                        topic: Some(reply_topic),
                                        is_request: false,
                                        timestamp: chrono::Utc::now().timestamp_millis() as u64,
                                    },
                                    Some(user_id) => {
                                        tracing::info!("User authenticated with ID: {}", user_id);
                                        let token = "".into();
                                        Message {
                                            id: Uuid::new_v4(),
                                            body: MessageBody::PersistenceQueryResponse(
                                                QueryResponse::AuthSuccess(token),
                                            ),
                                            topic: Some(reply_topic),
                                            is_request: false,
                                            timestamp: chrono::Utc::now().timestamp_millis() as u64,
                                        }
                                    }
                                };

                                if let Err(e) = broker.send(reply).await {
                                    tracing::error!(
                                        error = e.to_string(),
                                        "Failed to send persistence query response"
                                    );
                                }
                            }
                        }
                    }
                    _ => {
                        tracing::warn!(
                            "Persistence handler received unexpected message body: {:?}",
                            msg.body
                        );
                    }
                }
            }
        });

        Ok(handle)
    }
}
