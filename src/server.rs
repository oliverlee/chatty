use crate::board::{Board, Cell, Game, Player};
use crate::{ConnectionMessage, ManagerMessage, Result};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tracing::info;

pub async fn run(host: String) -> Result<()> {
    let (manager_tx, mut manager_rx) = mpsc::channel(2);

    let manager = tokio::spawn(async move {
        let mut game = Game::new();

        let mut connections =
            HashMap::<std::net::SocketAddr, mpsc::Sender<ConnectionMessage>>::new();
        let mut exiting = false;

        let mut available_players: Vec<Player> = vec![Player::X, Player::O];

        while let Some(msg) = manager_rx.recv().await {
            use ConnectionMessage as CM;
            use ManagerMessage as MM;
            info!("manager received {:?}", msg);
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
                        connections.insert(address, tx);

                        let tx = connections.get(&address).unwrap();

                        // TODO: don't clone maybe?
                        let _ = tx.send(CM::Game(game.clone())).await;
                        let _ = tx.send(CM::SetPlayer(available_players.pop())).await;

                        info!("{:?}", connections);
                    }
                }
                MM::Disconnect(address, player) => {
                    connections.remove(&address);

                    if let Some(p) = player {
                        available_players.push(p);
                    }

                    info!("{:?}", connections);
                    if exiting && connections.is_empty() {
                        break;
                    }
                }
                MM::Move(player, mov) => {
                    // is it my turn?
                    if let Ok(_) = game.try_move(player, mov.row, mov.col) {
                        for (_, tx) in connections.iter() {
                            let _ = tx.send(CM::Game(game.clone())).await;
                        }
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
        _ = run_accept_loop(host, manager_tx) => {}
    };

    manager.await?;

    Ok(())
}

async fn run_accept_loop(host: String, manager_tx: mpsc::Sender<ManagerMessage>) -> Result<()> {
    let addr = host.parse::<std::net::SocketAddr>()?;
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
    socket: tokio::net::TcpStream,
    address: std::net::SocketAddr,
    mut rx: mpsc::Receiver<ConnectionMessage>,
    manager_tx: mpsc::Sender<ManagerMessage>,
) {
    let (reader, writer) = socket.into_split();

    // Delimit frames using a length header
    let reader =
        tokio_util::codec::FramedRead::new(reader, tokio_util::codec::LengthDelimitedCodec::new());
    let writer =
        tokio_util::codec::FramedWrite::new(writer, tokio_util::codec::LengthDelimitedCodec::new());

    let mut reader = tokio_serde::SymmetricallyFramed::new(
        reader,
        tokio_serde::formats::SymmetricalCbor::<crate::Frame>::default(),
    );

    let mut writer = tokio_serde::SymmetricallyFramed::new(
        writer,
        tokio_serde::formats::SymmetricalCbor::<crate::Frame>::default(),
    );

    use futures::sink::SinkExt;

    let mut player = None;

    loop {
        tokio::select! {
            res = rx.recv() => {
                let msg = match res {
                    Some(msg) => msg,
                    None => break,
                };

                info!("Sending {:?} to {}", msg, address);
                match msg {
                    ConnectionMessage::Exit => {
                        break;
                    }
                    ConnectionMessage::SetPlayer(p) => player = p,
                    ConnectionMessage::Game(game) => {
                        if let Err(e) = writer.send(crate::Frame::Game(game)).await {
                            eprintln!("ouch: {:?}", e);
                            break;
                        }
                    }
                }
            }
            res = reader.try_next() => {
                info!("Received {:?}", res);

                // TODO : parse Move
                match res {
                    Ok(None) => {
                        // No more messages in stream
                        break;
                    }
                    Ok(Some(crate::Frame::Move(req))) => {
                        info!("yoyoyo {:?}", player);
                        if let Some(p) = player {
                            manager_tx.send(ManagerMessage::Move(p, req)).await.unwrap();
                        }
                        // TODO tell client its stupid
                    },
                    Ok(_) => {
                        eprintln!("Unspupporte3d!");
                    },
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
        .send(ManagerMessage::Disconnect(address, player))
        .await
        .unwrap();
}
