use crate::board::{Board, Cell, Player, Game};
use crate::{ConnectionMessage, ManagerMessage, Result};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::info;
use tokio_stream::StreamExt;

pub async fn run() -> Result<()> {
    let (manager_tx, mut manager_rx) = mpsc::channel(2);

    let manager = tokio::spawn(async move {
        let mut game = Game::new();

        let mut connections =
            HashMap::<std::net::SocketAddr, mpsc::Sender<ConnectionMessage>>::new();
        let mut exiting = false;

        let mut available_players: Vec<Player> = vec![Player::X, Player::O];
        let mut address_to_player: HashMap::<std::net::SocketAddr, Player> = HashMap::new();

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
                        // TODO: don't clone maybe?
                        let _ = tx.send(CM::Game(game.clone())).await;
                        connections.insert(address, tx);
                        // can we assign player?
                        if let Some(p) = available_players.pop() {
                            address_to_player.insert(address, p);
                        }
                        info!("{:?}", connections);
                    }
                }
                MM::Disconnect(address) => {
                    connections.remove(&address);
                    
                    if let Some(p) = address_to_player.remove(&address) {
                        available_players.push(p);
                    }

                    info!("{:?}", connections);
                    if exiting && connections.is_empty() {
                        break;
                    }
                }
                MM::Move(address, mov) => {
                    // figure otu player
                    if let Some(&p) = address_to_player.get(&address) {
                        // is it my turn?
                        if game.current_player == p {
                            // is the tile empty?
                            let cell = &mut game.board[(mov.row, mov.col)].0;
                            if cell.is_none() {
                                *cell = Some(p);

                                // we did something

                                // broadcast new board state
                                for (_, tx) in connections.iter() {
                                    let _ = tx.send(CM::Game(game.clone())).await;
                                }
                            }
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
    socket: tokio::net::TcpStream,
    address: std::net::SocketAddr,
    mut rx: mpsc::Receiver<ConnectionMessage>,
    manager_tx: mpsc::Sender<ManagerMessage>,
) {
    let (reader, writer) = socket.into_split();

    // Delimit frames using a length header
    let reader = tokio_util::codec::FramedRead::new(reader, tokio_util::codec::LengthDelimitedCodec::new());
    let writer = tokio_util::codec::FramedWrite::new(writer, tokio_util::codec::LengthDelimitedCodec::new());

    let mut reader = tokio_serde::SymmetricallyFramed::new(
        reader,
        tokio_serde::formats::SymmetricalCbor::<crate::Frame>::default(),
    );
    
    let mut writer = tokio_serde::SymmetricallyFramed::new(
        writer,
        tokio_serde::formats::SymmetricalCbor::<crate::Frame>::default(),
    );

    use futures::sink::SinkExt;

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
                    ConnectionMessage::Game(game) => {
                        if let Err(e) = writer.send(crate::Frame::Game(game)).await {
                            eprintln!("ouch: {:?}", e);
                            break;
                        }
                    }
                }
            }
            res = reader.try_next() => {
                // TODO : parse Move
                match res {
                    Ok(None) => {
                        // No more messages in stream
                        break;
                    }
                    Ok(Some(crate::Frame::Move(req))) => {
                        manager_tx.send(ManagerMessage::Move(address, req)).await.unwrap();
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
        .send(ManagerMessage::Disconnect(address))
        .await
        .unwrap();
}
