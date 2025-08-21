use regex::Regex;
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
