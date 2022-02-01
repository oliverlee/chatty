use crate::board::{Board, Cell, Player};
use crate::Result;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use tracing::info;

pub enum Control {
    Exit,
    Move { row: usize, col: usize },
}

pub async fn run(host: String) -> Result<()> {
    info!("Starting client...");

    let (tx, mut rx) = tokio::sync::mpsc::channel(2);

    tokio::spawn({
        let tx = tx.clone();
        async move {
            tokio::signal::ctrl_c().await.unwrap();
            let _ = tx.send(Control::Exit).await;
        }
    });

    std::thread::Builder::new()
        .name("stdin".into())
        .spawn({
            let tx = tx.clone();
            move || {
                let stdin = std::io::stdin();
                let mut stdin = stdin.lock();

                for line in std::io::BufRead::lines(&mut stdin) {
                    let line = line.unwrap();

                    let mut parts = line.split_ascii_whitespace();

                    let action = match parts.next().unwrap() {
                        "quit" => break,
                        "move" => {
                            let row = parts.next().map(str::parse::<usize>);
                            let col = parts.next().map(str::parse::<usize>);
                            match (row, col) {
                                (Some(Ok(row)), Some(Ok(col))) => Some(Control::Move { row, col }),
                                _ => {
                                    println!("move usage: move row col");
                                    None
                                }
                            }
                        }
                        _ => {
                            eprintln!("usage: move|quit");
                            None
                        }
                    };

                    if let Some(action) = action {
                        if let Err(_) = tx.blocking_send(action) {
                            eprintln!("failed to send line");
                        }
                    }
                }
            }
        })
        .unwrap();

    let socket = tokio::net::TcpStream::connect(host).await?;

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
    use tokio_stream::StreamExt;

    tokio::spawn({
        async move {
            while let Some(frame) = reader.try_next().await.unwrap() {
                match frame {
                    crate::Frame::Game(game) => {
                        println!("{}", game.board);
                    }
                    _ => unimplemented!(),
                }
            }
        }
    });

    while let Some(msg) = rx.recv().await {
        match msg {
            Control::Exit => break,
            Control::Move { row, col } => {
                let frame = crate::Frame::Move(crate::MoveRequest { row, col });
                info!("Sending frame {:?}", frame);
                writer.send(frame).await.unwrap();
                writer.flush().await.unwrap();
            }
        }
    }

    Ok(())
}
