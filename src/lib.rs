pub mod board;
pub mod client;
pub mod server;

use board::Game;
use tokio::sync::mpsc;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
pub enum ManagerMessage {
    Exit,
    Connect(std::net::SocketAddr, mpsc::Sender<ConnectionMessage>),
    Disconnect(std::net::SocketAddr),
    Move(std::net::SocketAddr, MoveRequest),
    Broadcast(ConnectionMessage),
}

#[derive(Debug, Clone)]
pub enum ConnectionMessage {
    Exit,
    Game(Game),
}

pub async fn shutdown(tx: mpsc::Sender<Option<String>>) {
    tokio::signal::ctrl_c().await.unwrap();
    tx.send(None).await.unwrap();
}

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Deserialize, Serialize)]
pub enum Frame {
    Game(board::Game),
    Move(MoveRequest),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MoveRequest {
    row: usize,
    col: usize,
}
