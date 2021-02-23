use tokio_tutorial::*;
use tracing::{info, error};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use std::sync::{Arc, Mutex};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub async fn server() -> Result<()> {
    info!("Starting server...");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:1234").await?;

    info!("Listening for connections on {}", listener.local_addr()?);

    let shared_memory = Arc::new(Mutex::new(vec![0u8; 256]));

    tokio::pin! {
        let shutdown = tokio::signal::ctrl_c();
    }

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                info!("Initiating graceful shutdown...");
                break;
            },
            res = listener.accept() => {
                let (socket, _) = res?;

                tokio::spawn(Connection {
                    socket,
                    shared_memory: Arc::clone(&shared_memory),
                }.handle());
            }
        }
    }

    Ok(())
}

struct Connection {
    socket: tokio::net::TcpStream,
    shared_memory: Arc<Mutex<Vec<u8>>>,
}

impl Connection {
    async fn handle(mut self) {
        let Self { socket, shared_memory } = &mut self;
        let peer_addr = socket.peer_addr().unwrap();
        info!("Accepted connection from {}", peer_addr);

        loop {
            let frame = match Frame::deserialize(socket).await {
                Ok(frame) => {
                    info!("Received frame {:?} from {}", &frame, peer_addr);
                    frame
                }
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => break,
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionAborted => break,
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => {
                    error!("Failed to read frame: {:?}", e);
                    break;
                }
            };

            match frame {
                Frame::ReadRequest(ReadRequest { address, count }) => {
                    let bytes = {
                        let start = address as usize;
                        let end = address as usize + count as usize;
                        
                        let mem = shared_memory.lock().unwrap();
                        if end > mem.len() {
                            // TODO: respond with error instead
                            Vec::new()
                        } else {
                            mem[start..end].to_vec()
                        }
                    };

                    let res = Frame::ReadResponse(ReadResponse { bytes });
                    info!("Sending response {:?}", &res);
                    match res.serialize(socket).await {
                        Ok(_) => {
                            info!("Sent response {:?}", &res);
                        },
                        Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => break,
                        Err(e) if e.kind() == std::io::ErrorKind::ConnectionAborted => break,
                        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                        Err(e) => {
                            error!("Failed to write frame: {:?}", e);
                            break;
                        }
                    }
                },
                Frame::WriteRequest(WriteRequest { address, bytes }) => {
                    let done = {
                        let start = address as usize;
                        let end = address as usize + bytes.len();
                        let mut mem = shared_memory.lock().unwrap();
                        if end > mem.len() {
                            false
                        } else {
                            mem[start..end].copy_from_slice(&bytes);
                            true
                        }
                    };

                    // TODO: respond with error, don't hold the lock


                    let res = Frame::WriteResponse(WriteResponse {});
                    info!("Sending response {:?}", &res);
                    match res.serialize(socket).await {
                        Ok(_) => {
                            info!("Sent response {:?}", &res);
                        },
                        Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => break,
                        Err(e) if e.kind() == std::io::ErrorKind::ConnectionAborted => break,
                        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                        Err(e) => {
                            error!("Failed to write frame: {:?}", e);
                            break;
                        }
                    }
                },
                _ => unimplemented!(),
            }
        }

        info!("Dropping connection with {}", peer_addr);
    }
}