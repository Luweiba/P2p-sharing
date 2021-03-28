use std::net::{Ipv4Addr, TcpListener, SocketAddr};

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
    pub fn new(local_bind_ip: Ipv4Addr, open_port: u16) -> Self {
        let listener = TcpListener::bind(SocketAddr::from((local_bind_ip.octets(), open_port))).unwrap();
        Self {
            open_port,
            local_bind_ip,
            p2p_listener: listener,
        }
    }
}