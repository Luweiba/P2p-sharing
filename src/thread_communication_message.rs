use crate::types::PeerInfo;
use std::net::SocketAddr;
use tokio::net::TcpStream;

/// local file manager to peer info manager
#[derive(Debug)]
pub struct LFToPIMessage {}
/// p2t manager to peer info manager
#[derive(Debug)]
pub enum P2TToPIMessage {
    // True => update
    // False => set
    AddOnePeerInfo {
        peer_info: PeerInfo,
    },
    UpdatePeerInfo {

    },
}

impl P2TToPIMessage {
    pub fn new_add_one_peer_info(peer_info: PeerInfo) -> Self {
        Self::AddOnePeerInfo {
            peer_info
        }
    }
}
/// t2p manager to peer info manager
#[derive(Debug)]
pub enum T2PToPIMessage {
    // True => update
    // False => set
    AddOnePeerInfo {
        peer_info: PeerInfo,
        peer_info_sync_open_port: u16,
    },
    UpdatePeerInfo {

    },
}

impl T2PToPIMessage {
    pub fn new_add_one_peer_info(peer_info: PeerInfo, peer_info_sync_open_port: u16) -> Self {
        Self::AddOnePeerInfo {
            peer_info,
            peer_info_sync_open_port,
        }
    }
}
/// p2p manager to peer info manager
pub struct P2PToPIMessage {

}
