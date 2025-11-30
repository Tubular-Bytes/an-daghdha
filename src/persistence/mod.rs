use std::sync::{Arc, RwLock};

use diesel::{r2d2::ConnectionManager, PgConnection};
use r2d2::Pool;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use uuid::Uuid;

mod inventory_repository;
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
    Auth {
        username: String,
        password: String,
    },
    GetInventoryIds,
    GetInventoryForUser {
        user_id: Uuid,
    },
    CreateBuilding {
        inventory_id: Uuid,
        blueprint_slug: String,
    },
    ProgressBuildings {
        inventory_id: Uuid,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum QueryResponse {
    // Placeholder for actual response types
    AuthSuccess(String),
    AuthFailed(String),

    GetInventoryIds(Vec<Uuid>),
    GetInventoryIdsFailed(String),

    GetInventoryIdForUser(Uuid),
    GetInventoryIdForUserFailed(String),

    CreateBuilding(Uuid),
    CreateBuildingFailed(String),
}

impl Default for PersistenceHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl PersistenceHandler {
    pub fn new() -> Self {
        Self {
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
                        tracing::info!(query = ?query, "querying persistence data with query");

                        match query {
                            Query::GetInventoryIds => {
                                PersistenceHandler::get_inventory_ids(conn, &broker, reply_topic)
                                    .await;
                            }
                            Query::GetInventoryForUser { user_id } => {
                                PersistenceHandler::get_inventory_id_for_user(
                                    conn,
                                    &broker,
                                    reply_topic,
                                    user_id,
                                )
                                .await;
                            }
                            Query::Auth { username, password } => {
                                PersistenceHandler::auth_user(
                                    conn,
                                    &broker,
                                    reply_topic,
                                    (&username, &password),
                                )
                                .await;
                            }
                            Query::CreateBuilding {
                                inventory_id,
                                blueprint_slug,
                            } => {
                                PersistenceHandler::create_building(
                                    conn,
                                    &broker,
                                    reply_topic,
                                    inventory_id,
                                    blueprint_slug,
                                )
                                .await;
                            }
                            Query::ProgressBuildings { inventory_id } => {
                                PersistenceHandler::progress_buildings(
                                    conn,
                                    &broker,
                                    reply_topic,
                                    inventory_id,
                                )
                                .await;
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

    pub async fn get_inventory_ids(
        conn: &mut PgConnection,
        broker: &MessageBroker,
        reply_topic: String,
    ) {
        tracing::debug!("received GetInventoryIds query");
        let reply_body = match user_repository::get_inventory_ids(conn).await {
            Ok(ids) => MessageBody::PersistenceQueryResponse(QueryResponse::GetInventoryIds(ids)),
            Err(e) => MessageBody::PersistenceQueryResponse(QueryResponse::GetInventoryIdsFailed(
                e.to_string(),
            )),
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

    pub async fn get_inventory_id_for_user(
        conn: &mut PgConnection,
        broker: &MessageBroker,
        reply_topic: String,
        user_id: Uuid,
    ) {
        tracing::debug!("received GetInventoryIdForUser query");
        let reply_body = match user_repository::get_inventory_id_for_user(conn, user_id).await {
            Ok(ids) => {
                MessageBody::PersistenceQueryResponse(QueryResponse::GetInventoryIdForUser(ids))
            }
            Err(e) => MessageBody::PersistenceQueryResponse(
                QueryResponse::GetInventoryIdForUserFailed(e.to_string()),
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

    pub async fn auth_user(
        conn: &mut PgConnection,
        broker: &MessageBroker,
        reply_topic: String,
        user_data: (&String, &String),
    ) {
        let id = user_repository::authenticate(conn, user_data.0, user_data.1)
            .await
            .unwrap_or(None);

        let reply: MessageBody = match id {
            None => MessageBody::PersistenceQueryResponse(QueryResponse::AuthFailed(
                "Authentication failed".into(),
            )),
            Some(user_id) => {
                tracing::info!("User authenticated with ID: {}", user_id);

                match crate::auth::token::generate_token(&user_id.to_string()) {
                    Ok(token) => {
                        MessageBody::PersistenceQueryResponse(QueryResponse::AuthSuccess(token))
                    }
                    Err(e) => MessageBody::PersistenceQueryResponse(QueryResponse::AuthFailed(
                        format!("Token generation failed: {}", e),
                    )),
                }
            }
        };

        let reply = Message::new(reply, Some(reply_topic), false);
        if let Err(e) = broker.send(reply).await {
            tracing::error!(
                error = e.to_string(),
                "Failed to send persistence query response"
            );
        }
    }

    pub async fn create_building(
        conn: &mut PgConnection,
        broker: &MessageBroker,
        reply_topic: String,
        inventory_id: Uuid,
        blueprint_slug: String,
    ) {
        tracing::debug!("received CreateBuilding query");

        let reply: MessageBody =
            match inventory_repository::create_building(conn, inventory_id, blueprint_slug.clone())
                .await
            {
                Ok(id) => MessageBody::PersistenceQueryResponse(QueryResponse::CreateBuilding(id)),
                Err(e) => MessageBody::PersistenceQueryResponse(
                    QueryResponse::CreateBuildingFailed(e.to_string()),
                ),
            };

        let reply = Message::new(reply, Some(reply_topic), false);
        if let Err(e) = broker.send(reply).await {
            tracing::error!(
                error = e.to_string(),
                "Failed to send persistence query response"
            );
        }
    }

    pub async fn progress_buildings(
        conn: &mut PgConnection,
        broker: &MessageBroker,
        reply_topic: String,
        inventory_id: Uuid,
    ) {
        let reply: MessageBody =
            match inventory_repository::process_building_ticks(conn, inventory_id).await {
                Ok(_) => {
                    // No specific response needed for progress operation
                    MessageBody::PersistenceQueryResponse(QueryResponse::CreateBuilding(
                        inventory_id,
                    ))
                }
                Err(e) => MessageBody::PersistenceQueryResponse(
                    QueryResponse::CreateBuildingFailed(e.to_string()),
                ),
            };

        let reply = Message::new(reply, Some(reply_topic), false);
        if let Err(e) = broker.send(reply).await {
            tracing::error!(
                error = e.to_string(),
                "Failed to send persistence query response"
            );
        }
    }
}
