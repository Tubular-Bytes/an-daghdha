use an_daghdha::actor::auth::AuthActorHandler;
use an_daghdha::messaging::{
    broker::MessageBroker, model::Message as InternalMessage, model::MessageBody, model::Status,
};
use an_daghdha::persistence::HandlerStatus;
use an_daghdha::{auth, AppState};
use axum::extract::ws::Message;
use axum::extract::{ws::WebSocket, State, WebSocketUpgrade};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use diesel::r2d2::ConnectionManager;
use diesel::PgConnection;
use r2d2::Pool;
use serde_json::json;
use tokio::signal;

use an_daghdha::auth::token;
use an_daghdha::websocket::api;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let (broker, mut handler) = MessageBroker::new();

    let task_handler = tokio::spawn(async move {
        handler.start().await;
    });

    let bouncer = api::Bouncer::new(&broker);

    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL environment variable not set"))?;

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder().build(manager)?;

    let mut persistence_handler = an_daghdha::persistence::PersistenceHandler::new();
    let persistence_handle = persistence_handler.listen(&broker, &pool).await?;

    for i in 0..5 {
        tracing::debug!(
            "[{}/5]waiting for persistence handler to start listening...",
            i + 1
        );
        match persistence_handler.status.read() {
            Ok(s) => {
                if *s == HandlerStatus::Listening {
                    break;
                }
            }
            Err(e) => {
                tracing::error!("Failed to read persistence handler status: {}", e);
                continue;
            }
        };

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    let auth_actor = an_daghdha::actor::auth::AuthActorHandler::new();

    let auth_broker = broker.clone();
    tokio::spawn(async move {
        auth_actor.listen(auth_broker).await.unwrap();
    });

    let inventory_ids = AuthActorHandler::get_inventory_ids(&broker).await;

    tracing::info!("Starting inventory actors for IDs: {:?}", inventory_ids);

    // todo graceful shutdown for inventory actors
    for inventory_id in inventory_ids {
        let inventory_actor =
            an_daghdha::actor::inventory::InventoryActorHandler { id: inventory_id };
        let inventory_broker = broker.clone();
        tokio::spawn(async move {
            inventory_actor.listen(inventory_broker).await.unwrap();
        });
    }

    let state = (bouncer, broker.clone()).into();

    let app = Router::new()
        .route("/health", get(|| async { Json(json!({ "status": "ok" })) }))
        .route("/auth/login", post(auth::handler::handle_login))
        .route("/rtc", get(ws_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();

    // Wait for Ctrl+C signal
    match signal::ctrl_c().await {
        Ok(()) => {
            tracing::info!("Received Ctrl+C, initiating graceful shutdown...");
        }
        Err(err) => {
            tracing::error!("Unable to listen for shutdown signal: {}", err);
            // Continue anyway to allow manual shutdown
        }
    }

    // Send stop signal to the bus
    if let Err(e) = broker
        .send(InternalMessage::new(MessageBody::Stop, None, false))
        .await
    {
        tracing::error!("Failed to send stop signal to bus: {}", e);
    }

    let mut grace_wait = 5;
    loop {
        let status = broker.status().await;
        if status == Status::Stopped {
            break;
        }

        grace_wait -= 1;

        if grace_wait <= 0 {
            tracing::error!("failed to wait for broker to shut down gracefully, terminating");
            break;
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    task_handler.await?;
    persistence_handle.await?;
    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, headers, state))
}

async fn handle_socket(mut ws: WebSocket, headers: HeaderMap, state: AppState) {
    tracing::info!("New WebSocket connection with headers {:?}", headers);

    let user_id = match token::validate_headers(&headers) {
        Ok(uid) => uid,
        Err(e) => {
            tracing::error!("Failed to validate headers: {}", e);
            ws.send(Message::Text("Unauthorized".into()))
                .await
                .unwrap_or_else(|e| {
                    tracing::error!("Failed to send unauthorized message: {}", e);
                });
            return;
        }
    };

    state.bouncer.handle_connection(user_id, ws).await;
}
