use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::persistence::{Query, QueryResponse};

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Unstarted,
    Running,
    Stopping,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PersistenceRecord {
    // Placeholder for actual record types
    Dummy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageBody {
    AuthenticationRequest {
        user: String,
        password: String,
    },
    AuthenticationResponse(Result<String, String>),

    BuildRequest {
        inventory_id: Uuid,
        blueprint_id: String,
    },
    BuildResponse(Result<String, String>),

    DebugMessage(String),

    PersistenceQueryRequest(Query),
    PersistenceQueryResponse(QueryResponse),

    Stop,
    Empty,
}

impl MessageBody {
    pub fn from_value(value: &Value) -> Result<Self, anyhow::Error> {
        let kind = value
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("Missing 'kind' field in message body"))?;

        match kind {
            "authentication" => {
                tracing::debug!("Parsing authentication request: {value:?}");

                let body = value
                    .get("body")
                    .and_then(Value::as_object)
                    .ok_or_else(|| {
                        anyhow::anyhow!("Missing 'body' field in authentication message")
                    })?;

                let user = body.get("user").and_then(Value::as_str).unwrap_or_default();
                let password = body
                    .get("password")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                Ok(Self::AuthenticationRequest {
                    user: user.into(),
                    password: password.into(),
                })
            }
            "build" => {
                tracing::debug!("Parsing build request: {value:?}");

                let body = value
                    .get("body")
                    .and_then(Value::as_object)
                    .ok_or_else(|| anyhow::anyhow!("Missing 'body' field in build message"))?;

                let inventory_id = body
                    .get("inventory_id")
                    .and_then(Value::as_str)
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .ok_or_else(|| {
                        anyhow::anyhow!("Missing or invalid 'inventory_id' in build message")
                    })?;

                let blueprint_id = body
                    .get("blueprint_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default();

                Ok(Self::BuildRequest {
                    inventory_id,
                    blueprint_id: blueprint_id.into(),
                })
            }
            _ => Err(anyhow::anyhow!("Unknown message body kind: {}", kind)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub body: MessageBody,
    pub topic: Option<String>,
    pub is_request: bool,
    pub timestamp: u64,
}

impl Message {
    pub fn new(body: MessageBody, topic: Option<String>, is_request: bool) -> Self {
        Self {
            id: Uuid::new_v4(),
            body,
            topic,
            is_request,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
        }
    }

    pub fn new_request(body: MessageBody, topic: Option<String>) -> Self {
        Self::new(body, topic, true)
    }

    pub fn reply_topic(&self) -> String {
        format!("reply-{}", self.id)
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
        Ok((
            Self {
                id: Uuid::new_v4(),
                pattern: regex,
                tx,
            },
            rx,
        ))
    }
}
