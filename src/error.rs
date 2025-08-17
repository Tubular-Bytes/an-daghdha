use tokio_tungstenite::tungstenite;
use std::fmt::Display;

#[derive(Debug)]
pub enum ApiError {
    WebsocketError(tungstenite::Error),
}

impl From<tungstenite::Error> for ApiError {
    fn from(err: tungstenite::Error) -> Self {
        ApiError::WebsocketError(err)
    }
}

impl Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::WebsocketError(err) => write!(f, "WebSocket error: {}", err),
        }
    }
}