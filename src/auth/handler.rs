use axum::{extract::State, Json};
use serde_json::json;

use crate::{auth::model::AuthRequest, messaging::model::Message, AppState};

pub async fn handle_login(
    State(state): State<AppState>,
    Json(payload): Json<AuthRequest>,
) -> Json<serde_json::Value> {
    tracing::info!("Login attempt for user: {}", payload.user);

    let user = state.broker.request(Message::new(
        crate::messaging::model::MessageBody::AuthenticationRequest { user: payload.user, password: payload.password },
        Some("auth".to_string()),
        true,
    )).await;

    match user {
        Ok(response) => {
            if response.is_none() {
                tracing::warn!("Authentication failed: No response received");
                return Json(json!({ "error": "Authentication failed" }));
            }

            match response.unwrap().body {
                crate::messaging::model::MessageBody::AuthenticationResponse(result) => {
                    match result {
                        Ok(token) => {
                            tracing::info!("Authentication successful");
                            Json(json!({ "token": token }))
                        }
                        Err(err) => {
                            tracing::warn!("Authentication failed: {}", err);
                            Json(json!({ "error": "Authentication failed" }))
                        }
                    }
                }
                _ => {
                    tracing::warn!("Unexpected message body in authentication response");
                    Json(json!({ "error": "Authentication failed" }))
                }
            }
        }
        Err(err) => {
            tracing::warn!("Authentication failed: {}", err);
            Json(json!({ "error": "Authentication failed" }))
        }
    }
}