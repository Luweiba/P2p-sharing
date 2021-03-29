use crate::thread_communication_message::{LFToPIMessage, P2PToPIMessage, P2TToPIMessage};
use crate::types::PeerInfo;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use uuid::Uuid;
pub struct PeersInfoTable {
    // 全局PeersInfo Table
    peers_info: Vec<PeerInfo>,
    // sha3_code => Uuid 进行映射，用于全局去重，uuid的作用是防止同名的文件以及内容相同但文件名不同的文件
    files_info_hash_map: HashMap<Vec<u8>, Uuid>, // 全局表, 与本地的表进行同步， //TODO 或许local_file_manager不需要此字段（！！还是需要同步的）
}

impl PeersInfoTable {
    pub fn new() -> Self {
        Self {
            peers_info: Vec::new(),
            files_info_hash_map: HashMap::new(),
        }
    }

    // 修改peers_info信息
    pub fn update(&mut self, update_command: PeersInfoUpdateCommand) {}

    // 处理接收的peerInfo信息
    pub fn add_peer_info(&mut self, peer_info: PeerInfo) {}
}

/// 全局PeersInfo信息的去重与实时更新
pub struct PeersInfoManager {
    // 全局的PeersInfo Table
    peers_info_table: Arc<Mutex<PeersInfoTable>>, // 与local_file_manager的通信（：本地文件信息变更与本地文件信息与全局信息同步）
                                                  // local_file_manager_sender: Sender<LFToPIMessage>,
                                                  // local_file_manager_receiver: Receiver<LFToPIMessage>,
                                                  // 与p2t_manager的通信(：peerInfo push，peerInfo pull）
                                                  // peer_to_tracker_manager_sender: Sender<P2TToPIMessage>,
                                                  // peer_to_tracker_manager_receiver: Receiver<P2TToPIMessage>,
                                                  // 与p2p_manager的通信(：接收文件请求与制定请求策略）
                                                  // peer_to_peer_manager_sender: Sender<P2PToPIMessage>,
                                                  // peer_to_peer_manager_receiver: Receiver<P2PToPIMessage>
}

impl PeersInfoManager {
    pub async fn start(
        &mut self,
        lf_sender: Sender<LFToPIMessage>,
        mut lf_receiver: Receiver<LFToPIMessage>,
        p2t_sender: Sender<P2TToPIMessage>,
        mut p2t_receiver: Receiver<P2TToPIMessage>,
        p2p_sender: Sender<P2PToPIMessage>,
        mut p2p_receiver: Receiver<P2PToPIMessage>,
    ) -> Result<(), Box<dyn Error>> {
        // TODO 处理通信逻辑
        // lf_receiver
        tokio::spawn(async move {
            loop {
                if let Some(msg) = lf_receiver.recv().await {}
            }
        })
        .await?;
        // p2t_receiver
        tokio::spawn(async move {
            loop {
                if let Some(msg) = p2t_receiver.recv().await {}
            }
        })
        .await?;
        // p2p_receiver
        tokio::spawn(async move {
            loop {
                if let Some(msg) = p2p_receiver.recv().await {}
            }
        })
        .await?;
        Ok(())
    }

    pub fn new(peers_info_table: PeersInfoTable) -> Self {
        Self {
            peers_info_table: Arc::new(Mutex::new(peers_info_table)),
        }
    }
}

pub struct PeersInfoUpdateCommand {}
