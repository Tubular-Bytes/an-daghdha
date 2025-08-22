use futures_channel::mpsc::UnboundedSender;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::{
    handshake::server::{Request, Response},
    protocol::Message,
};

use crate::messaging::{broker::MessageBroker, model::Message as BusMessage, model::MessageBody};

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

lazy_static::lazy_static! {
    static ref EMPTY_MESSAGE: BusMessage = BusMessage::new(MessageBody::Empty, None, false);
}

#[derive(Clone)]
pub struct Bouncer {
    peers: PeerMap,
    broker: MessageBroker,
}

impl Bouncer {
    pub fn new(broker: &MessageBroker) -> Self {
        Bouncer {
            peers: Arc::new(Mutex::new(HashMap::new())),
            broker: broker.clone(),
        }
    }

    pub async fn handle_connection(self, addr: SocketAddr, stream: TcpStream) {
        tracing::debug!("handling incoming connection: {:?}", addr);

        let callback = |req: &Request, response: Response| {
            let headers = req.headers();
            tracing::debug!("got auth header: {:?}", headers.get("Authorization"));

            Ok(response)
        };

        let ws_stream = match tokio_tungstenite::accept_hdr_async(stream, callback).await {
            Ok(stream) => stream,
            Err(err) => {
                tracing::error!("Error during the websocket handshake: {}", err);
                return;
            }
        };
        tracing::debug!("webSocket connection established: {}", addr);

        let (mut write, mut read) = ws_stream.split();

        while let Some(msg) = read.next().await {
            match msg {
                Ok(msg) => {
                    if !msg.is_text() {
                        tracing::warn!("Received non-text message: {:?}", msg);
                        continue;
                    }
                    let response = self.handle_message(msg.clone()).await;
                    match response {
                        Ok(resp) => {
                            tracing::debug!("Sending response: {:?}", resp);
                            write.send(Message::text(format!("Response: {:?}", resp))).await.unwrap_or_else(|e| {
                                tracing::error!("Failed to send response: {}", e);
                            });
                        }
                        Err(e) => {
                            tracing::error!("Error handling message: {}", e);
                            let msg = Message::text(format!("Error: {}", e));
                            write.send(msg).await.unwrap_or_else(|e| {
                                tracing::error!("Failed to send error message: {}", e);
                            });
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Error receiving message: {}", e);
                    break;
                }
            }
        }

        tracing::debug!("{} disconnected", addr);
        self.peers.lock().unwrap().remove(&addr);
    }

    async fn handle_message(&self, message: Message) -> Result<BusMessage, anyhow::Error> {
        tracing::debug!("raw message: {message:?}");
        let msg: Value = serde_json::from_str(message.to_text()?)
            .map_err(|e| anyhow::anyhow!("Failed to parse message: {}", e))?;
        tracing::debug!("parsed message: {msg:?}");

        let message_body = MessageBody::from_value(&msg)?;

        match &message_body {
            MessageBody::AuthenticationRequest { .. } => {
                tracing::debug!("forwarding auth request to auth actor");
                let message = BusMessage::new(message_body, topic("auth"), true);

                match self.broker.request(message).await {
                    Ok(response) => {
                        tracing::debug!("Received response from auth actor: {:?}", response);
                        return Ok(response.unwrap());
                    }
                    Err(e) => {
                        tracing::error!("Error sending message to bus: {}", e);
                        return Err(anyhow::anyhow!("Error sending message to bus: {}", e));
                    }
                }
            }
            _ => {
                tracing::warn!("unexpected message body: {:?}", message_body);
                return Err(anyhow::anyhow!("unexpected message body"));
            }
        }
    }
}

pub fn topic(name: &str) -> Option<String> {
    match name {
        "" | "none" => None,
        _ => Some(format!("topic:{}", name)),
    }
}