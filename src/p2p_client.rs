

use crate::types::{PeerInfo, FileMetaInfo, FilePieceInfo, PeersInfoTable};
use std::net::{SocketAddr, Ipv4Addr};
use std::net::{TcpListener, TcpStream};
use std::io::Write;
use std::path::PathBuf;
use crate::message::Message;
use crate::peer_to_tracker::PeerToTrackerManager;
use crate::peer_to_peer::PeerToPeerManager;
use crate::file_manager::LocalFileManager;


#[derive(Debug)]
pub struct P2PClient {
    // 全局唯一的Peer信息表（peer_id, peer_ip, peer_open_port, peer_file_meta_info_report)
    peers_info: Option<PeersInfoTable>,
    /// local File meta info report ( `may change when get message from tracker` )
    local_file_manager: LocalFileManager,
    /// Peer to Tracker manager
    peer_to_tracker_manager: Option<PeerToTrackerManager>,
    /// Peer to Peer
    peer_to_peer_manager: PeerToPeerManager,
}

impl P2PClient {
    pub fn new(local_bind_ip: Ipv4Addr, open_port: u16, local_base_directory: PathBuf, piece_size: u32) -> Self {
        let peer_to_peer_manager = PeerToPeerManager::new(local_bind_ip, open_port);
        let mut local_file_manager = LocalFileManager::new(local_base_directory, piece_size);
        local_file_manager.initialize().expect("local file manager initialize failed");
        Self {
            peers_info: None,
            local_file_manager,
            peer_to_tracker_manager: None,
            peer_to_peer_manager,
        }
    }

    // 发送Register请求包
    pub fn connect_to_tracker(&mut self, tracker_ip: Ipv4Addr, tracker_port: u16) -> std::io::Result<()> {
        self.tracker_ip = Some(tracker_ip.clone());
        self.tracker_port = Some(tracker_port);
        self.peer_to_tracker_manager = Some(PeerToTrackerManager::new(tracker_ip, tracker_port));
        self.peer_to_tracker_manager.as_ref().unwrap()
        Ok(())
    }
}