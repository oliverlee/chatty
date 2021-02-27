use crate::board::{Board, Cell, Player};
use crate::Result;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use tracing::info;

pub async fn run() -> Result<()> {
    info!("starting client");

    let mut stream = tokio::net::TcpStream::connect("127.0.0.1:4321").await?;

    // Should receive the board state
    let mut bytes = [0u8; 13];
    stream.read_exact(&mut bytes).await?;

    assert_eq!(&[0x00, 0x00, 0x00, 0x01], &bytes[0..4]);

    let mut board = Board::default();
    for i in 0..9 {
        let r = i / 3;
        let c = i % 3;
        board.0[r][c] = Cell(match bytes[4 + i] {
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
                        board.0[row][col] = match p {
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
