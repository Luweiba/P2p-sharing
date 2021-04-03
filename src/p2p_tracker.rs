use crate::file_manager::LocalFileManager;
use crate::keep_alive_manager::KeepAliveManager;
use crate::peer_to_peer::PeerToPeerManager;
use crate::peer_to_tracker::TrackerToPeerManager;
use crate::peers_info_manager::PeersInfoManagerOfTracker;
use crate::thread_communication_message::{
    KAToPIMessage, LFToPIMessage, P2PToPIMessage, P2TToPIMessage, T2PToPIMessage,
};
use crate::types::PeersInfoTable;
use std::error::Error;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use tokio::sync::mpsc;

const CHANNEL_DEFAULT_SIZE: usize = 100;
pub struct P2PTracker {
    // 全局唯一的Peer信息表（peer_id, peer_ip, peer_open_port, peer_file_meta_info_report)
    // peers_info: Option<PeersInfoManager>,
    // local File meta info report ( `may change when get message from tracker` )
    // local_file_manager: LocalFileManager,
    // Peer to Tracker manager
    // tracker_to_peer_manager: TrackerToPeerManager,
    // Peer to Peer
    // peer_to_peer_manager: PeerToPeerManager,
    tracker_ip: Ipv4Addr,
    tracker_port: u16,
    interval: u32,
    open_port: u16,
    local_base_directory: PathBuf,
    piece_size: u32,
    scanning_interval: u64,
    keep_alive_interval: u64,
    keep_alive_open_port: u16,
}

impl P2PTracker {
    pub fn new(
        tracker_ip: Ipv4Addr,
        tracker_port: u16,
        interval: u32,
        open_port: u16,
        local_base_directory: PathBuf,
        piece_size: u32,
        scanning_interval: u64,
        keep_alive_interval: u64,
        keep_alive_open_port: u16,
    ) -> Self {
        Self {
            tracker_ip,
            tracker_port,
            interval,
            open_port,
            local_base_directory,
            piece_size,
            scanning_interval,
            keep_alive_interval,
            keep_alive_open_port,
        }
    }
    // Tracker_to_peer 通信的目的是为了统一全局的peer信息表
    pub async fn start_tracking(&mut self) -> Result<(), Box<dyn Error>> {
        println!("Start Tracking ...");
        // 创建线程间通信channel
        let (lf2pi_sender, lf2pi_receiver) = mpsc::channel::<LFToPIMessage>(CHANNEL_DEFAULT_SIZE);
        let (pi2lf_sender, pi2lf_receiver) = mpsc::channel::<LFToPIMessage>(CHANNEL_DEFAULT_SIZE);
        let (t2p2pi_sender, t2p2pi_receiver) =
            mpsc::channel::<T2PToPIMessage>(CHANNEL_DEFAULT_SIZE);
        let (pi2t2p_sender, pi2t2p_receiver) =
            mpsc::channel::<T2PToPIMessage>(CHANNEL_DEFAULT_SIZE);
        let (p2p2pi_sender, p2p2pi_receiver) =
            mpsc::channel::<P2PToPIMessage>(CHANNEL_DEFAULT_SIZE);
        let (pi2p2p_sender, pi2p2p_receiver) =
            mpsc::channel::<P2PToPIMessage>(CHANNEL_DEFAULT_SIZE);
        let (ka2pi_sender, ka2pi_receiver) = mpsc::channel::<KAToPIMessage>(CHANNEL_DEFAULT_SIZE);
        let (pi2ka_sender, pi2ka_receiver) = mpsc::channel::<KAToPIMessage>(CHANNEL_DEFAULT_SIZE);
        // P2P
        println!("Initialize P2P manager ...");
        let peer_to_peer_manager = PeerToPeerManager::new(self.tracker_ip, self.open_port).await?;
        println!("Initialize T2P manager ...");
        // 初始化本地文件信息与实时更新模块
        let mut local_file_manager = LocalFileManager::new_and_initialize(
            self.local_base_directory.clone(),
            self.piece_size,
        )
        .await?;
        local_file_manager
            .start_scaning_and_updating_periodically(
                self.scanning_interval,
                lf2pi_sender,
                pi2lf_receiver,
            )
            .await;
        // 启动全局peers_info信息表管理模块
        let mut peer_info_manager = PeersInfoManagerOfTracker::new(PeersInfoTable::tracker_new(
            self.tracker_ip.octets().to_vec(),
            self.open_port,
            local_file_manager.get_file_meta_info_report().await,
        ));
        peer_info_manager
            .start(
                self.keep_alive_interval,
                self.keep_alive_open_port,
                pi2lf_sender,
                lf2pi_receiver,
                pi2t2p_sender,
                t2p2pi_receiver,
                pi2p2p_sender,
                p2p2pi_receiver,
                pi2ka_sender,
                ka2pi_receiver,
            )
            .await?;
        // 启动KeepAlive模块
        let mut keep_alive_manager = KeepAliveManager::new(
            self.tracker_ip.clone(),
            self.keep_alive_interval,
            self.keep_alive_open_port,
        );
        tokio::spawn(async move {
            keep_alive_manager
                .start_monitoring(ka2pi_sender, pi2ka_receiver)
                .await;
        });
        // 启动Tracker准备接受Peer的连接
        let mut tracker_to_peer_manager =
            TrackerToPeerManager::new(self.tracker_ip, self.tracker_port, self.interval);
        tracker_to_peer_manager
            .start(t2p2pi_sender, pi2t2p_receiver)
            .await?;
        Ok(())
    }
}
