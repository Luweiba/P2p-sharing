use crate::types::PeerInfo;
use std::net::{Ipv4Addr, SocketAddr};
use std::net::TcpStream;


/// 负责管理Peer与Tracker的通信
/// 主要包括：注册、更新、保持联系
pub struct PeerToTrackerManager {
    // peer_id 用于标识一个对等方，Tracker的peer_id为 0
    peer_id: Option<u32>,
    // 本机用于Peer to Tracker的TCPStream
    p2t_client_stream: TcpStream,
    // tracker IP
    tracker_ip: Ipv4Addr,
    // tracker port
    tracker_port: u16,
    // 局域网内IP
    routable_ip: Option<Ipv4Addr>,
    // 保持通信的间隔(ms)
    interval: Option<u32>,
}

impl PeerToTrackerManager {
    pub fn new(tracker_ip: Ipv4Addr, tracker_port: u16) -> Self {
        let client_stream = TcpStream::connect(SocketAddr::from((tracker_ip.octets(), tracker_port))).expect("Unable to connect to the tracker");
        Self {
            peer_id: None,
            p2t_client_stream: client_stream,
            tracker_ip,
            tracker_port,
            routable_ip: None,
            interval: None,
        }
    }

    pub fn register_to_tracker(&mut self, ) -> std::io::Result<()> {

        Ok(())
    }
}

pub struct TrackerToPeerManager {

}