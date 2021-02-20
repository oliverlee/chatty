use tokio::sync::mpsc;
use tracing::{info};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = std::env::args();

    match args.skip(1).next().as_ref().map(String::as_ref) {
        Some("client") | None => run_client().await?,
        Some("server") => run_server().await?,
        _ => panic!("Invalid command, specify client|server")
    };

    Ok(())
}

async fn run_client() -> Result<()> {
    info!("starting client");

    let (tx, rx) = mpsc::channel(32);

    let _ = std::thread::Builder::new()
        .name("stdin".into())
        .spawn({
            let tx = tx.clone();
            move || {
                let stdin = std::io::stdin();
                let mut stdin = stdin.lock();

                for line in std::io::BufRead::lines(&mut stdin) {
                    tx.blocking_send(Some(line.unwrap())).unwrap();
                }
            }
        });

    tokio::spawn(shutdown(tx));

    run_client_loop(rx).await.unwrap();

    Ok(())
}

async fn run_server() -> Result<()> {
    info!("starting server");

    Ok(())
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
                    },
                    _ => println!("X|O row col"),
                }
            }
            other => println!("{:?} {}", std::time::Instant::now(), other),
        }
    }

    info!("stopping run");

    Ok(())
}

use std::fmt;

#[derive(Debug)]
enum Player {
    X,
    O,
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Default)]
struct Cell(Option<Player>);

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0.as_ref() {
            Some(player) => player.fmt(f),
            None => " ".fmt(f),
        }
    }
}

#[derive(Debug, Default)]
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
