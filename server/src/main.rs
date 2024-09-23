use anyhow::{anyhow, Result};
use proto::runnel::tunnel_server::{Tunnel, TunnelServer};
use proto::runnel::{TunnelRequest, TunnelResponse};
use tonic::{transport::Server, Request, Response, Status};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tokio::sync::mpsc;

#[derive(Default)]
pub struct CoreTunnelServer {}

#[tonic::async_trait]
impl Tunnel for CoreTunnelServer {
    type TunnelMessageStream = ReceiverStream<Result<TunnelResponse, Status>>;

    async fn tunnel_message(
        &self,
        request: Request<tonic::Streaming<TunnelRequest>>,
    ) -> Result<Response<Self::TunnelMessageStream>, Status> {
        println!("服务器收到请求：{:?}", request.remote_addr());

        let mut req_stream = request.into_inner();
        let (tx, rx) = mpsc::channel(1000);

        async_stream::try_stream! {
              while let Some(result) = req_stream.next().await {
                let result = result.unwrap();
                println!("Message recieved: {:?}", result.message);
                tx.send(Ok( { TunnelResponse{data: vec![]} })).await.unwrap();
            }
        };

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "[::1]:50051".parse().unwrap();
    let core_server = CoreTunnelServer::default();

    println!("Rust Tunnel listening on {}", addr);

    Server::builder()
        .add_service(TunnelServer::new(core_server))
        .serve(addr)
        .await?;

    Ok(())
}