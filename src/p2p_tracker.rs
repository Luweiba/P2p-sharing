use crate::file_manager::LocalFileManager;
use crate::peer_to_peer::PeerToPeerManager;
use crate::peer_to_tracker::TrackerToPeerManager;
use crate::peers_info_manager::PeersInfoManager;
use std::error::Error;
use std::net::Ipv4Addr;
use std::path::PathBuf;

pub struct P2PTracker {
    // 全局唯一的Peer信息表（peer_id, peer_ip, peer_open_port, peer_file_meta_info_report)
    peers_info: Option<PeersInfoManager>,
    /// local File meta info report ( `may change when get message from tracker` )
    local_file_manager: LocalFileManager,
    /// Peer to Tracker manager
    tracker_to_peer_manager: TrackerToPeerManager,
    /// Peer to Peer
    peer_to_peer_manager: PeerToPeerManager,
}

impl P2PTracker {
    pub async fn new(
        tracker_ip: Ipv4Addr,
        tracker_port: u16,
        interval: u32,
        local_bind_ip: Ipv4Addr,
        open_port: u16,
        local_base_directory: PathBuf,
        piece_size: u32,
    ) -> Result<Self, Box<dyn Error>> {
        let peer_to_peer_manager = PeerToPeerManager::new(local_bind_ip, open_port).await?;
        let tracker_to_peer_manager = TrackerToPeerManager::new(tracker_ip, tracker_port, interval);
        let mut local_file_manager = LocalFileManager::new(local_base_directory, piece_size);
        local_file_manager
            .initialize()
            .expect("local file manager initialize failed");

        Ok(Self {
            peers_info: None,
            local_file_manager,
            tracker_to_peer_manager,
            peer_to_peer_manager,
        })
    }
    // Tracker_to_peer 通信的目的是为了统一全局的peer信息表
    pub fn start_tracking(&mut self) {
        self.tracker_to_peer_manager.start();
    }
}
