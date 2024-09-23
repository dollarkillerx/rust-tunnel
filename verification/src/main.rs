mod main_udp;

use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use bytes::BytesMut;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("0.0.0.0:6080").await?;
    println!("SOCKS5 server listening on 0.0.0.0:6080");

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("New connection from {}", addr);
        tokio::spawn(async move {
            if let Err(e) = handle_client(socket).await {
                eprintln!("Error handling client {}: {:?}", addr, e);
            }
        });
    }
}

async fn handle_client(mut client: TcpStream) -> Result<(), Box<dyn Error>> {
    // 1. 握手阶段
    let mut buf = [0u8; 2];
    client.read_exact(&mut buf).await?;
    if buf[0] != 0x05 {
        return Err("Unsupported SOCKS version".into());
    }
    let n_methods = buf[1] as usize;
    let mut methods = vec![0u8; n_methods];
    client.read_exact(&mut methods).await?;
    // 只支持不认证
    if !methods.contains(&0x00) {
        // 回复不支持的认证方法
        client.write_all(&[0x05, 0xFF]).await?;
        return Err("No acceptable authentication methods".into());
    }
    // 回复选择不认证
    client.write_all(&[0x05, 0x00]).await?;

    // 2. 请求阶段
    let mut header = [0u8; 4];
    client.read_exact(&mut header).await?;
    if header[0] != 0x05 {
        return Err("Unsupported SOCKS version in request".into());
    }
    let cmd = header[1];
    let _reserved = header[2];
    let addr_type = header[3];

    // 解析目标地址
    let dest_addr = match addr_type {
        0x01 => { // IPv4
            let mut addr = [0u8; 4];
            client.read_exact(&mut addr).await?;
            let ip = std::net::Ipv4Addr::from(addr);
            ip.to_string()
        },
        0x03 => { // 域名
            let mut len = [0u8; 1];
            client.read_exact(&mut len).await?;
            let mut addr = vec![0u8; len[0] as usize];
            client.read_exact(&mut addr).await?;
            String::from_utf8(addr)?
        },
        0x04 => { // IPv6
            let mut addr = [0u8; 16];
            client.read_exact(&mut addr).await?;
            let ip = std::net::Ipv6Addr::from(addr);
            ip.to_string()
        },
        _ => return Err("Unsupported address type".into()),
    };

    // 解析目标端口
    let mut port_bytes = [0u8; 2];
    client.read_exact(&mut port_bytes).await?;
    let port = u16::from_be_bytes(port_bytes);

    println!("Request to connect to {}:{}", dest_addr, port);

    // 只支持 CONNECT 命令
    if cmd != 0x01 {
        send_reply(&mut client, 0x07, &"0.0.0.0", 0).await?;
        return Err("Command not supported".into());
    }

    // 尝试建立目标连接
    let target = format!("{}:{}", dest_addr, port);
    match TcpStream::connect(target).await {
        Ok(mut target_stream) => {
            // 获取本地地址信息用于回复客户端
            let local_addr = target_stream.local_addr()?;
            send_reply(&mut client, 0x00, &local_addr.ip().to_string(), local_addr.port()).await?;

            // 3. 数据转发
            let (mut ri, mut wi) = client.split();
            let (mut ro, mut wo) = target_stream.split();

            // 双向转发
            let client_to_target = tokio::io::copy(&mut ri, &mut wo);
            let target_to_client = tokio::io::copy(&mut ro, &mut wi);

            tokio::try_join!(client_to_target, target_to_client)?;
        },
        Err(_) => {
            // 连接失败
            send_reply(&mut client, 0x05, &"0.0.0.0", 0).await?;
            return Err("Failed to connect to target".into());
        }
    }

    Ok(())
}

async fn send_reply(client: &mut TcpStream, rep: u8, bind_addr: &str, bind_port: u16) -> Result<(), Box<dyn Error>> {
    let mut reply = vec![0x05, rep, 0x00];

    // 简单处理IPv4
    if let Ok(ip) = bind_addr.parse::<std::net::Ipv4Addr>() {
        reply.push(0x01);
        reply.extend_from_slice(&ip.octets());
    } else if let Ok(ip) = bind_addr.parse::<std::net::Ipv6Addr>() {
        reply.push(0x04);
        reply.extend_from_slice(&ip.octets());
    } else { // 域名
        let addr = bind_addr.as_bytes();
        reply.push(0x03);
        reply.push(addr.len() as u8);
        reply.extend_from_slice(addr);
    }

    reply.extend_from_slice(&bind_port.to_be_bytes());

    client.write_all(&reply).await?;
    Ok(())
}
