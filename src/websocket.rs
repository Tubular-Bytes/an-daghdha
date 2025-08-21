use futures_channel::mpsc::UnboundedSender;
use futures_util::StreamExt;
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

use crate::messaging::{
    broker::MessageBroker,
    model::Message as BusMessage, model::MessageBody
};

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

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

        let mut ws_stream = match tokio_tungstenite::accept_hdr_async(stream, callback).await {
            Ok(stream) => stream,
            Err(err) => {
                tracing::error!("Error during the websocket handshake: {}", err);
                return;
            }
        };
        tracing::debug!("webSocket connection established: {}", addr);

        // Insert the write part of this peer to the peer map.
        let (tx, _rx) = futures_channel::mpsc::unbounded();
        self.peers.lock().unwrap().insert(addr, tx);

        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(msg) => {
                    if let Err(e) = self.handle_message(msg.clone()).await {
                        tracing::error!("Error handling message: {}", e);
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

    async fn handle_message(&self, message: Message) -> Result<(), anyhow::Error> {
        if !message.is_text() {
            tracing::warn!("received non-text message: {message:?}");
            return Ok(());
        }

        tracing::debug!("raw message: {message:?}");
        let msg: Value = serde_json::from_str(message.to_text()?)
            .map_err(|e| anyhow::anyhow!("Failed to parse message: {}", e))?;
        tracing::debug!("parsed message: {msg:?}");

        let message = BusMessage::new(
            MessageBody::Example { foo: msg.to_string() },
            None,
            true,
        );

        if let Err(e) = self.broker.send(message).await {
            tracing::error!("Error sending message to bus: {}", e);
        }

        Ok(())
    }
}
