mod client;
mod server;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = std::env::args();
    match args.skip(1).next().as_ref().map(String::as_str) {
        Some("client") | None => client::client().await?,
        Some("server") => server::server().await?,
        _ => panic!("specify client or server as a command line argument."),
    }
    Ok(())
}
