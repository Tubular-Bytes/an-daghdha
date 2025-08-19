#[derive(Clone)]
pub enum Message {
    Shutdown,
    Authentication {
        username: String,
        password: String,
        // Instant reply
        reply: std::sync::Arc<tokio::sync::oneshot::Sender<AuthenticationResponse>>,
    },
    Authorization {
        token: String,
        reply: std::sync::Arc<tokio::sync::oneshot::Sender<AuthorizationResponse>>,
    },
    Tick,
    Task {
        task: Task,
        source: (),
    },
    Set {
        key: String,
        value: String, // Switch this to dynamic value when it's implemented
        reply: Option<std::sync::Arc<tokio::sync::oneshot::Sender<SetResponse>>>,
    },
    Get {
        key: String,
        reply: Option<std::sync::Arc<tokio::sync::oneshot::Sender<GetResponse>>>,
    },
    Delete {
        key: String,
        reply: Option<std::sync::Arc<tokio::sync::oneshot::Sender<DeleteResponse>>>,
    },
    Example(String),
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::Shutdown => write!(f, "Message::Shutdown"),
            Message::Authentication {
                username,
                password: _,
                reply: _,
            } => {
                write!(f, "Message::Authentication {{ username: {:?}, password: [REDACTED], reply: [Sender] }}", username)
            }
            Message::Authorization { token: _, reply: _ } => {
                write!(
                    f,
                    "Message::Authorization {{ token: [REDACTED], reply: [Sender] }}"
                )
            }
            Message::Tick => write!(f, "Message::Tick"),
            Message::Task { task, source } => {
                write!(
                    f,
                    "Message::Task {{ task: {:?}, source: {:?} }}",
                    task, source
                )
            }
            Message::Set {
                key,
                value,
                reply: _,
            } => {
                write!(
                    f,
                    "Message::Set {{ key: {:?}, value: {:?}, reply: [Sender] }}",
                    key, value
                )
            }
            Message::Get { key, reply: _ } => {
                write!(f, "Message::Get {{ key: {:?}, reply: [Sender] }}", key)
            }
            Message::Delete { key, reply: _ } => {
                write!(f, "Message::Delete {{ key: {:?}, reply: [Sender] }}", key)
            }
            Message::Example(example) => {
                write!(f, "Message::Example({:?})", example)
            }
        }
    }
}

pub struct AuthenticationResponse {
    pub success: bool,
    pub message: String,
}

pub struct AuthorizationResponse {
    pub authorized: bool,
    pub message: String,
}

#[derive(Clone, Debug)]
pub enum Task {
    Build(BuildTask),
    Train(TrainTask),
    Produce(ProduceTask),
}

#[derive(Clone, Debug)]
pub struct BuildTask {
    pub inventory_id: String,
    pub blueprint_id: String,
}

#[derive(Clone, Debug)]
pub struct TrainTask {
    pub inventory_id: String,
    pub formula_id: String,
}

#[derive(Clone, Debug)]
pub struct ProduceTask {
    pub inventory_id: String,
    pub recipe_id: String,
}

#[derive(Clone)]
pub struct SetResponse {
    pub result: Result<(), String>,
}

#[derive(Clone)]
pub struct GetResponse {
    pub value: Result<String, String>, // Replace with dynamic value
}

#[derive(Clone)]
pub struct DeleteResponse {
    pub result: Result<(), String>,
}
