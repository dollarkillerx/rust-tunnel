use anyhow::{Context, Result};

#[derive(Default)]
pub struct Tunnel {}

#[tonic::async_trait]
impl TunnelServer for Tunnel {

}

#[tokio::main]
async fn main() -> Result<()> {
    Ok(())
}

