use regex::Regex;
use serde_json::Value;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Unstarted,
    Running,
    Stopping,
    Stopped,
}

#[derive(Debug, Clone)]
pub enum MessageBody {
    Example { name: String },
    ExampleResponse { name: String },

    AuthenticationRequest { user: String, password: String },
    AuthenticationResponse(Result<String, String>),

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
            "example" => {
                let name = value
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                Ok(Self::Example { name: name.into() })
            }
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
            _ => Err(anyhow::anyhow!("Unknown message body kind: {}", kind)),
        }
    }
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
    pub fn new(body: MessageBody, topic: Option<String>, is_request: bool) -> Self {
        Self {
            id: Uuid::new_v4(),
            body,
            topic,
            is_request,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
        }
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
