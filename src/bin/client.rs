use anyhow::{Context, Result};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:6080").await?;
    println!("SOCKS5 server listening on 0.0.0.0:6080");

    Ok(())
}