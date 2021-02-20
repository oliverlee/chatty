use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = std::env::args();

    match args.skip(1).next().as_ref().map(String::as_ref) {
        Some("client") | None => run_client().await?,
        Some("server") => run_server().await?,
        _ => panic!("Invalid command, specify client|server"),
    };

    Ok(())
}

async fn run_client() -> Result<()> {
    info!("starting client");

    let mut stream = tokio::net::TcpStream::connect("0.0.0.0:4321").await?;

    // Should receive the board state
    let mut bytes = [0u8; 13];
    stream.read_exact(&mut bytes).await?;

    assert_eq!(&[0x00, 0x00, 0x00, 0x01], &bytes[0..4]);

    let mut board = Board::default();
    for i in 0..9 {
        let r = i/3;
        let c = i%3;
        board.cells[r][c] = Cell(match bytes[4 + i] {
            0x00 => None,
            0x01 => Some(Player::X),
            0x02 => Some(Player::O),
            other => panic!("Invalid board cell byte {:?}", other),
        });
    }

    println!("{}", board);

    // let (tx, rx) = mpsc::channel(32);

    // let _ = std::thread::Builder::new().name("stdin".into()).spawn({
    //     let tx = tx.clone();
    //     move || {
    //         let stdin = std::io::stdin();
    //         let mut stdin = stdin.lock();

    //         for line in std::io::BufRead::lines(&mut stdin) {
    //             tx.blocking_send(Some(line.unwrap())).unwrap();
    //         }
    //     }
    // });

    // tokio::spawn(shutdown(tx));

    // run_client_loop(rx).await.unwrap();

    Ok(())
}

#[derive(Debug, Clone)]
enum ManagerMessage {
    Exit,
    Connect(std::net::SocketAddr, mpsc::Sender<ConnectionMessage>),
    Disconnect(std::net::SocketAddr),
    Broadcast(ConnectionMessage),
}

#[derive(Debug, Clone)]
enum ConnectionMessage {
    Exit,
    Board(Board),
}

async fn run_server() -> Result<()> {
    let (manager_tx, mut manager_rx) = mpsc::channel(2);

    let manager = tokio::spawn(async move {
        let mut board = Board::default();
        board.cells[0][1] = Cell(Some(Player::X));
        board.cells[1][1] = Cell(Some(Player::X));
        board.cells[2][1] = Cell(Some(Player::X));
        board.cells[2][0] = Cell(Some(Player::O));
        board.cells[2][2] = Cell(Some(Player::O));

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
        _ = run_server_loop(manager_tx) => {}
    };

    manager.await?;

    Ok(())
}

async fn run_server_loop(manager_tx: mpsc::Sender<ManagerMessage>) -> Result<()> {
    let addr = "0.0.0.0:4321".parse::<std::net::SocketAddr>()?;
    info!("Attempting to bind to {}...", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("Accepting connections from {}...", listener.local_addr()?);
    loop {
        let (socket, address) = listener.accept().await?;
        info!("Accepted connection from {}.", address);

        let (tx, rx) = mpsc::channel::<ConnectionMessage>(2);

        tokio::spawn(run_server_connection_loop(
            socket,
            address,
            rx,
            manager_tx.clone(),
        ));

        manager_tx
            .send(ManagerMessage::Connect(address, tx.clone()))
            .await?;
    }
}

async fn run_server_connection_loop(
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
                            bytes[4 + i] = match board.cells[r][c].0 {
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

async fn shutdown(tx: mpsc::Sender<Option<String>>) {
    tokio::signal::ctrl_c().await.unwrap();
    tx.send(None).await.unwrap();
}

async fn run_client_loop(mut rx: mpsc::Receiver<Option<String>>) -> Result<()> {
    let mut board = Board::default();

    loop {
        println!("{}", board);

        let line = match rx.recv().await {
            Some(Some(line)) => line,
            _ => break,
        };

        let mut parts = line.split_ascii_whitespace();

        match parts.next().unwrap() {
            "quit" => break,
            p if p == "X" || p == "O" => {
                let row = parts.next().map(str::parse::<usize>);
                let col = parts.next().map(str::parse::<usize>);
                match (row, col) {
                    (Some(Ok(row)), Some(Ok(col))) => {
                        board.cells[row][col] = match p {
                            "X" => Cell(Some(Player::X)),
                            "O" => Cell(Some(Player::O)),
                            _ => panic!("what the hell"),
                        };
                    }
                    _ => println!("usage: X|O row col"),
                }
            }
            other => println!("{:?} {}", std::time::Instant::now(), other),
        }
    }

    info!("stopping run");

    Ok(())
}

use std::fmt;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Player {
    X,
    O,
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
struct Cell(Option<Player>);

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0.as_ref() {
            Some(player) => player.fmt(f),
            None => " ".fmt(f),
        }
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
struct Board {
    cells: [[Cell; 3]; 3],
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "┬───┬───┬───┬")?;
        for rowi in 0..3 {
            let row = &self.cells[rowi];
            writeln!(f, "│ {} │ {} │ {} │", row[0], row[1], row[2])?;
            if rowi + 1 < 3 {
                writeln!(f, "├───┼───┼───┼")?;
            }
        }
        writeln!(f, "├───┴───┴───┘")?;
        Ok(())
    }
}

struct RawKind(u32);

impl RawKind {
    pub const EXIT: Self = Self(0);
    pub const BOARD: Self = Self(1);
}

struct RawBoard {
    pub cells: [[u8; 3]; 3],
}
