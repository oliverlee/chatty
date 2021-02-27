use crate::board::{Board, Cell, Player};
use crate::{ConnectionMessage, ManagerMessage, Result};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::info;

pub async fn run() -> Result<()> {
    let (manager_tx, mut manager_rx) = mpsc::channel(2);

    let manager = tokio::spawn(async move {
        let mut board = Board::default();
        board.0[0][1] = Cell(Some(Player::X));
        board.0[1][1] = Cell(Some(Player::X));
        board.0[2][1] = Cell(Some(Player::X));
        board.0[2][0] = Cell(Some(Player::O));
        board.0[2][2] = Cell(Some(Player::O));

        let mut connections =
            HashMap::<std::net::SocketAddr, mpsc::Sender<ConnectionMessage>>::new();
        let mut exiting = false;

        while let Some(msg) = manager_rx.recv().await {
            use ConnectionMessage as CM;
            use ManagerMessage as MM;
            match msg {
                MM::Exit => {
                    if connections.is_empty() {
                        break;
                    }
                    exiting = true;
                    for (_, tx) in connections.iter() {
                        let _ = tx.send(CM::Exit).await;
                    }
                }
                MM::Connect(address, tx) => {
                    if !exiting {
                        let _ = tx.send(CM::Board(board.clone())).await;
                        connections.insert(address, tx);
                        info!("{:?}", connections);
                    }
                }
                MM::Disconnect(address) => {
                    connections.remove(&address);
                    info!("{:?}", connections);
                    if exiting && connections.is_empty() {
                        break;
                    }
                }
                MM::Broadcast(message) => {
                    for (_, tx) in connections.iter() {
                        let _ = tx.send(message.clone()).await;
                    }
                }
            }
        }
    });

    let shutdown_manager_tx = manager_tx.clone();
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Initiating graceful shutdown...");
            shutdown_manager_tx.send(ManagerMessage::Exit).await?;
        },
        _ = run_accept_loop(manager_tx) => {}
    };

    manager.await?;

    Ok(())
}

async fn run_accept_loop(manager_tx: mpsc::Sender<ManagerMessage>) -> Result<()> {
    let addr = "0.0.0.0:4321".parse::<std::net::SocketAddr>()?;
    info!("Attempting to bind to {}...", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("Accepting connections from {}...", listener.local_addr()?);
    loop {
        let (socket, address) = listener.accept().await?;
        info!("Accepted connection from {}.", address);

        let (tx, rx) = mpsc::channel::<ConnectionMessage>(2);

        tokio::spawn(run_connection_loop(socket, address, rx, manager_tx.clone()));

        manager_tx
            .send(ManagerMessage::Connect(address, tx.clone()))
            .await?;
    }
}

async fn run_connection_loop(
    mut socket: tokio::net::TcpStream,
    address: std::net::SocketAddr,
    mut rx: mpsc::Receiver<ConnectionMessage>,
    manager_tx: mpsc::Sender<ManagerMessage>,
) {
    let socket = &mut socket;

    loop {
        let mut buffer = vec![0; 4096];

        tokio::select! {
            res = rx.recv() => {
                let msg = match res {
                    Some(msg) => msg,
                    None => break,
                };

                info!("Sending {:?} to {}", msg, address);
                match msg {
                    ConnectionMessage::Exit => {
                        let bytes: [u8; 4] = [0x00, 0x00, 0x00, 0x00];
                        if let Err(_) = tokio::io::AsyncWriteExt::write_all(socket, &bytes).await {
                            break;
                        }
                    }
                    ConnectionMessage::Board(board) => {
                        let mut bytes = [0u8; 13];
                        bytes[3] = 0x01;
                        for i in 0..9 {
                            let r = i / 3;
                            let c = i % 3;
                            bytes[4 + i] = match board.0[r][c].0 {
                                Some(Player::X) => 0x01,
                                Some(Player::O) => 0x02,
                                None => 0x00,
                            };
                        }
                        if let Err(_) = tokio::io::AsyncWriteExt::write_all(socket, &bytes).await {
                            break;
                        }
                    }
                }
            }
            res = tokio::io::AsyncReadExt::read_buf(socket, &mut buffer) => {
                match res {
                    Ok(0) => break,
                    Ok(n) => {
                        println!("received bytes from client {:?}", &buffer[0..n])
                    }
                    Err(e) => {
                        eprintln!("error read {:?}", e);
                        break;
                    }
                }
            }
        }
    }

    info!("Disconnected from {}.", address);

    manager_tx
        .send(ManagerMessage::Disconnect(address))
        .await
        .unwrap();
}
