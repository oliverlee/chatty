use tokio_tutorial::*;
use tracing::{info};
// use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use std::sync::{Arc,Mutex};
use std::collections::VecDeque;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub enum Control {
    Exit,
    Send(Frame),
}

pub async fn client() -> Result<()> {
    info!("Starting client...");

    let (tx, mut rx) = tokio::sync::mpsc::channel(2);
    let sent_requests: Arc<Mutex<VecDeque<Frame>>> = Default::default();
    
    tokio::spawn({
        let tx = tx.clone();
        async move {
            tokio::signal::ctrl_c().await.unwrap();
            let _ = tx.send(Control::Exit).await;
        }
    });

    std::thread::Builder::new().name("stdin".into()).spawn({
        let tx = tx.clone();
        move || {
            let stdin = std::io::stdin();
            let mut stdin = stdin.lock();

            for line in std::io::BufRead::lines(&mut stdin) {
                let line = line.unwrap();
                let mut parts = line.split_ascii_whitespace();
                match parts.next() {
                    Some("read") => {
                        let address = parts.next().map(str::parse::<u32>);
                        let count = parts.next().map(str::parse::<u32>);
                        match (address, count) {
                            (Some(Ok(address)), Some(Ok(count))) => {
                                let _ = tx.blocking_send(Control::Send(Frame::ReadRequest(ReadRequest { address, count })));
                            },
                            _ => println!("read usage: read <address> <count>"),
                        }
                    },
                    Some("write") => {
                        let address = parts.next().map(str::parse::<u32>);
                        let bytes = parts.next();
                        match (address, bytes) {
                            (Some(Ok(address)), Some(bytes)) => {
                                let _ = tx.blocking_send(Control::Send(Frame::WriteRequest(WriteRequest { address, bytes: bytes.as_bytes().to_vec() })));
                            }
                            _ => println!("write usage: write <address> <bytes>"),
                        }
                    }
                    _ => println!("usage: read | write ...args"),
                }
            }
        }
    }).unwrap();

    let stream = tokio::net::TcpStream::connect("127.0.0.1:1234").await?;
    let (mut reader, mut writer) = stream.into_split();

    tokio::spawn({
        async move {
            loop {
                let res = match Frame::deserialize(&mut reader).await {
                    Ok(res) => res,
                    Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => break,
                    Err(e) if e.kind() == std::io::ErrorKind::ConnectionAborted => break,
                    Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                    Err(e) => panic!("Failed to deserialize message: {:?}", e),
                };

                info!("received response: {:?}", res);
            }
        }
    });

    while let Some(msg) = rx.recv().await {
        match msg {
            Control::Exit => break,
            Control::Send(req) => {
                match req.serialize(&mut writer).await {
                    Ok(_) => {
                        info!("sent request: {:?}", &req);

                        // Should we submit this before sending? Can the reply in theory be read before this task is resumed after req.serialize.await finishes?
                        sent_requests.lock().unwrap().push_back(req);
                    },
                    Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => break,
                    Err(e) if e.kind() == std::io::ErrorKind::ConnectionAborted => break,
                    Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                    Err(e) => panic!("failed to serialize: {:?}", &e),
                }
            }
        }
    }

    Ok(())
}
