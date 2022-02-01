pub mod board;
pub mod client;
pub mod server;

use board::{Game, Player};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum ManagerMessage {
    Exit,
    Connect(std::net::SocketAddr, mpsc::Sender<ConnectionMessage>),
    Disconnect(std::net::SocketAddr, Option<Player>),
    Move(Player, MoveRequest),
    Broadcast(ConnectionMessage),
}

#[derive(Debug, Clone)]
pub enum ConnectionMessage {
    Exit,
    SetPlayer(Option<Player>),
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
