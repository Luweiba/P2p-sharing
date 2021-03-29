use std::error::Error;
use std::net::{Ipv4Addr, SocketAddr};
use tokio::net::TcpListener;
/// 用于管理对等方的通信
/// 主要包括：请求文件和应答文件
pub struct PeerToPeerManager {
    // 本机用于Peer to Peer的端口号
    open_port: u16,
    // 本机绑定IP
    local_bind_ip: Ipv4Addr,
    // 本机用于Peer to Peer的TCPListener
    p2p_listener: TcpListener,
}

impl PeerToPeerManager {
    pub async fn new(local_bind_ip: Ipv4Addr, open_port: u16) -> Result<Self, Box<dyn Error>> {
        let listener =
            TcpListener::bind(SocketAddr::from((local_bind_ip.octets(), open_port))).await?;
        Ok(Self {
            open_port,
            local_bind_ip,
            p2p_listener: listener,
        })
    }

    pub fn get_open_port(&self) -> u16 {
        self.open_port
    }
}
