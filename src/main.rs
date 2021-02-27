use chatty::{client, server, Result};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = std::env::args();

    match args.skip(1).next().as_ref().map(String::as_ref) {
        Some("client") | None => client::run().await?,
        Some("server") => server::run().await?,
        _ => panic!("Invalid command, specify client|server"),
    };

    Ok(())
}
