mod socks;
use tokio_stream::StreamExt;
use proto::runnel::tunnel_client::TunnelClient;
use anyhow::Result;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:6080").await?;
    println!("SOCKS5 server listening on 0.0.0.0:6080");
    let client = TunnelClient::connect("http://127.0.0.1:50051").await?;
    loop {
        let (socket, addr) = listener.accept().await?;
        let new_client = client.clone();
        println!("New connection from {}", addr);
        tokio::spawn(async move {
            if let Err(e) = socks::handle_socks5_server(socket,new_client).await {
                eprintln!("Error handling client {}: {:?}", addr, e);
            }
        });
    }
}