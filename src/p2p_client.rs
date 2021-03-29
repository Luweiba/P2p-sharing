use crate::file_manager::LocalFileManager;
use crate::peer_to_peer::PeerToPeerManager;
use crate::peer_to_tracker::PeerToTrackerManager;
use crate::peers_info_manager::PeersInfoManager;
use crate::thread_communication_message::{LFToPIMessage, P2PToPIMessage, P2TToPIMessage};
use crate::types::{FileMetaInfo, FilePieceInfo, PeerInfo};
use std::error::Error;
use std::io::Write;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

const CHANNEL_DEFAULT_SIZE: usize = 100;
pub struct P2PClient {
    local_bind_ip: Ipv4Addr,
    open_port: u16,
    local_base_directory: PathBuf,
    piece_size: u32,
    // 全局唯一的Peer信息表（peer_id, peer_ip, peer_open_port, peer_file_meta_info_report)
    // peers_info: Option<PeersInfoManager>,
    // local File meta info report ( `may change when get message from tracker` )
    // local_file_manager: LocalFileManager,
    // Peer to Tracker manager
    // peer_to_tracker_manager: Option<PeerToTrackerManager>,
    // Peer to Peer
    // peer_to_peer_manager: PeerToPeerManager,
}

impl P2PClient {
    pub fn new(
        local_bind_ip: Ipv4Addr,
        open_port: u16,
        local_base_directory: PathBuf,
        piece_size: u32,
    ) -> Self {
        Self {
            local_bind_ip,
            open_port,
            local_base_directory,
            piece_size,
        }
    }

    // 发送Register请求，并建立长久的通信联系
    pub async fn connect_to_tracker(
        &mut self,
        tracker_ip: Ipv4Addr,
        tracker_port: u16,
    ) -> Result<(), Box<dyn Error>> {
        // 创建线程间通信channel
        let (lf2pi_sender, lf2pi_receiver) = mpsc::channel::<LFToPIMessage>(CHANNEL_DEFAULT_SIZE);
        let (pi2lf_sender, pi2lf_receiver) = mpsc::channel::<LFToPIMessage>(CHANNEL_DEFAULT_SIZE);
        let (p2t2pi_sender, p2t2pi_receiver) =
            mpsc::channel::<P2TToPIMessage>(CHANNEL_DEFAULT_SIZE);
        let (pi2p2t_sender, pi2p2t_receiver) =
            mpsc::channel::<P2TToPIMessage>(CHANNEL_DEFAULT_SIZE);
        let (p2p2pi_sender, p2p2pi_receiver) =
            mpsc::channel::<P2PToPIMessage>(CHANNEL_DEFAULT_SIZE);
        let (pi2p2p_sender, pi2p2p_receiver) =
            mpsc::channel::<P2PToPIMessage>(CHANNEL_DEFAULT_SIZE);

        let peer_to_peer_manager =
            PeerToPeerManager::new(self.local_bind_ip, self.open_port).await?;
        let mut local_file_manager =
            LocalFileManager::new(self.local_base_directory.clone(), self.piece_size);
        local_file_manager
            .initialize()
            .expect("local file manager initialize failed");
        let mut peer_to_tracker_manager = PeerToTrackerManager::new(tracker_ip, tracker_port);
        peer_to_tracker_manager
            .register_to_tracker(
                local_file_manager.get_file_meta_info_report(),
                peer_to_peer_manager.get_open_port(),
                p2t2pi_sender,
                pi2p2t_receiver,
            )
            .await?;
        Ok(())
    }
}
