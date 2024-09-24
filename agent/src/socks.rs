use tokio::net::TcpStream;
use anyhow::{Result, anyhow};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(crate) async fn handle_socks5_server(mut client: TcpStream) -> Result<()> {
    // 1. Handshake phase
    let mut buf = [0u8; 2];
    client.read_exact(&mut buf).await?;
    if buf[0] != 0x05 {
        return Err(anyhow!("Unsupported SOCKS version"));
    }
    let n_methods = buf[1] as usize;
    let mut methods = vec![0u8; n_methods];
    client.read_exact(&mut methods).await?;
    // Only supports no authentication
    if !methods.contains(&0x00) {
        // Reply with unsupported authentication method
        client.write_all(&[0x05, 0xFF]).await?;
        return Err(anyhow!("No acceptable authentication methods"));
    }
    // Reply with no authentication selected
    client.write_all(&[0x05, 0x00]).await?;

    // 2. Request phase
    let mut header = [0u8; 4];
    client.read_exact(&mut header).await?;
    if header[0] != 0x05 {
        return Err(anyhow!("Unsupported SOCKS version in request"));
    }
    let cmd = header[1];
    let _reserved = header[2];
    let addr_type = header[3];

    // Parse destination address
    let dest_addr = match addr_type {
        0x01 => { // IPv4
            let mut addr = [0u8; 4];
            client.read_exact(&mut addr).await?;
            let ip = std::net::Ipv4Addr::from(addr);
            ip.to_string()
        }
        0x03 => { // Domain name
            let mut len = [0u8; 1];
            client.read_exact(&mut len).await?;
            let mut addr = vec![0u8; len[0] as usize];
            client.read_exact(&mut addr).await?;
            String::from_utf8(addr)?
        }
        0x04 => { // IPv6
            let mut addr = [0u8; 16];
            client.read_exact(&mut addr).await?;
            let ip = std::net::Ipv6Addr::from(addr);
            ip.to_string()
        }
        _ => return Err(anyhow!("Unsupported address type")),
    };

    // Parse destination port
    let mut port_bytes = [0u8; 2];
    client.read_exact(&mut port_bytes).await?;
    let port = u16::from_be_bytes(port_bytes);

    println!("Request to connect to {}:{}", dest_addr, port);

    // Only supports CONNECT command
    if cmd != 0x01 {
        send_reply(&mut client, 0x07, "0.0.0.0", 0).await?;
        return Err(anyhow!("Command not supported"));
    }

    // Attempt to establish connection to the target
    let target = format!("{}:{}", dest_addr, port);
    match TcpStream::connect(target).await {
        Ok(mut target_stream) => {
            // Get local address information for replying to the client
            let local_addr = target_stream.local_addr()?;
            send_reply(&mut client, 0x00, &local_addr.ip().to_string(), local_addr.port()).await?;

            // 3. Data forwarding
            let (mut ri, mut wi) = client.split();
            let (mut ro, mut wo) = target_stream.split();

            // Bidirectional forwarding
            let client_to_target = tokio::io::copy(&mut ri, &mut wo);
            let target_to_client = tokio::io::copy(&mut ro, &mut wi);

            tokio::try_join!(client_to_target, target_to_client)?;
        }
        Err(_) => {
            // Connection failed
            send_reply(&mut client, 0x05, "0.0.0.0", 0).await?;
            return Err(anyhow!("Failed to connect to target"));
        }
    }

    Ok(())
}

async fn send_reply(client: &mut TcpStream, rep: u8, bind_addr: &str, bind_port: u16) -> Result<()> {
    let mut reply = vec![0x05, rep, 0x00];

    // Handle IPv4 and IPv6 addresses
    if let Ok(ip) = bind_addr.parse::<std::net::Ipv4Addr>() {
        reply.push(0x01);
        reply.extend_from_slice(&ip.octets());
    } else if let Ok(ip) = bind_addr.parse::<std::net::Ipv6Addr>() {
        reply.push(0x04);
        reply.extend_from_slice(&ip.octets());
    } else { // Domain name
        let addr = bind_addr.as_bytes();
        reply.push(0x03);
        reply.push(addr.len() as u8);
        reply.extend_from_slice(addr);
    }

    reply.extend_from_slice(&bind_port.to_be_bytes());

    client.write_all(&reply).await?;
    Ok(())
}
