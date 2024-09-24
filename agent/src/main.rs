mod socks;

use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReceiverStream;
use tonic::Request;
use tonic::transport::Channel;
use proto::runnel::tunnel_client::TunnelClient;
use proto::runnel::TunnelRequest;
use anyhow::Result;
use tokio::net::TcpListener;
// let mut client = TunnelClient::connect("http://127.0.0.1:50051")
// .await.unwrap();
//
// handle_client(&mut client).await;
// async fn handle_client(client: &mut TunnelClient<Channel>) {
//     let mut last_seen_pong: u32 = 0;
//     let (tx, rx) = mpsc::channel(10000);
//     let ack = ReceiverStream::new(rx);
//
//     let response = client.tunnel_message(Request::new(ack)).await.unwrap();
//
//     let message = format!("last seen pong: {}", last_seen_pong);
//     // kick start the pingpong with an init tx.send
//     tx.send(TunnelRequest {
//         message: message.into_bytes(),
//         over: false,
//         target: "127.0.2.1".into(),
//     }).await.unwrap();
//     let mut stream = response.into_inner();
//     while let Some(result) = stream.next().await {
//         let result = result.unwrap();
//         println!("Message recieved: {:?}", String::from_utf8(result.data));
//         let message = format!("last seen pong: {}", last_seen_pong);
//         tx.send(TunnelRequest {
//             message: message.into_bytes(),
//             over: false,
//             target: "127.0.0.1".into(),
//         }).await.unwrap();
//     }
// }


#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:6080").await?;
    println!("SOCKS5 server listening on 0.0.0.0:6080");

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("New connection from {}", addr);
        tokio::spawn(async move {
            if let Err(e) = socks::handle_socks5_server(socket).await {
                eprintln!("Error handling client {}: {:?}", addr, e);
            }
        });
    }
}