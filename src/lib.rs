pub mod board;
pub mod client;
pub mod server;

use board::Board;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum ManagerMessage {
    Exit,
    Connect(std::net::SocketAddr, mpsc::Sender<ConnectionMessage>),
    Disconnect(std::net::SocketAddr),
    Broadcast(ConnectionMessage),
}

#[derive(Debug, Clone)]
pub enum ConnectionMessage {
    Exit,
    Board(Board),
}

pub async fn shutdown(tx: mpsc::Sender<Option<String>>) {
    tokio::signal::ctrl_c().await.unwrap();
    tx.send(None).await.unwrap();
}

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
