use chatty::{client, server, Result};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let mut args = std::env::args().skip(1);

    let is_client = match args.next().as_ref().map(String::as_ref) {
        Some("client") | None => true,
        Some("server") => false,
        _ => panic!("Invalid command, specify client|server"),
    };

    let host = args.next().unwrap_or_else(|| {
        if is_client {
            "127.0.0.1:1234".to_string()
        } else {
            "0.0.0.0:1234".to_string()
        }
    });

    match is_client {
        true => client::run(host).await?,
        false => server::run(host).await?,
    }

    Ok(())
}
