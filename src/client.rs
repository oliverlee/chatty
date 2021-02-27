use crate::board::{Board, Cell, Player};
use crate::Result;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use tracing::info;

pub async fn run() -> Result<()> {
    info!("starting client");

    let mut socket = tokio::net::TcpStream::connect("127.0.0.1:4321").await?;

    let (reader, writer) = socket.into_split();

    // Delimit frames using a length header
    let reader = tokio_util::codec::FramedRead::new(reader, tokio_util::codec::LengthDelimitedCodec::new());
    let writer = tokio_util::codec::FramedWrite::new(writer, tokio_util::codec::LengthDelimitedCodec::new());

    let mut reader = tokio_serde::SymmetricallyFramed::new(
        reader,
        tokio_serde::formats::SymmetricalCbor::<crate::Frame>::default(),
    );
    
    // let mut writer = tokio_serde::SymmetricallyFramed::new(
    //     writer,
    //     tokio_serde::formats::SymmetricalCbor::<crate::Frame>::default(),
    // );

    use futures::sink::SinkExt;
    use tokio_stream::StreamExt;

    while let Some(frame) = reader.try_next().await.unwrap() {
        println!("{:?}", frame);

    }


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
