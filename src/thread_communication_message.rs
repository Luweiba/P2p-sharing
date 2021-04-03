use crate::types::{FileMetaInfo, PeerInfo};
use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use tokio::net::TcpStream;

/// local file manager to peer info manager
/// 本地用于告知PeerInfoManager本地文件变动（TODO 暂时只有添加文件，不可删除文件
#[derive(Debug, Clone)]
pub struct LFToPIMessage {
    new_files_meta_info: Vec<FileMetaInfo>,
}

impl LFToPIMessage {
    pub fn new() -> Self {
        Self {
            new_files_meta_info: Vec::new(),
        }
    }

    pub fn add_file_meta_info(&mut self, file_meta_info: FileMetaInfo) {
        self.new_files_meta_info.push(file_meta_info);
    }

    pub fn get_inner_files_meta_info(&self) -> Vec<FileMetaInfo> {
        self.new_files_meta_info.clone()
    }
}
/// p2t manager to peer info manager
#[derive(Debug)]
pub enum P2TToPIMessage {
    // True => update
    // False => set
    AddOnePeerInfo {
        peer_info: PeerInfo,
    },
    UpdatePeerInfo {
        peer_id: u32,
        updated_file_meta_info: Vec<FileMetaInfo>,
    },
}

impl P2TToPIMessage {
    pub fn new_add_one_peer_info(peer_info: PeerInfo) -> Self {
        Self::AddOnePeerInfo { peer_info }
    }
    pub fn new_update_one_peer_info(
        peer_id: u32,
        updated_file_meta_info: Vec<FileMetaInfo>,
    ) -> Self {
        Self::UpdatePeerInfo {
            peer_id,
            updated_file_meta_info,
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
        peer_id: u32,
        updated_file_meta_info: Vec<FileMetaInfo>,
    },
}

impl T2PToPIMessage {
    pub fn new_add_one_peer_info(peer_info: PeerInfo, peer_info_sync_open_port: u16) -> Self {
        Self::AddOnePeerInfo {
            peer_info,
            peer_info_sync_open_port,
        }
    }

    pub fn new_update_one_peer_info(
        peer_id: u32,
        updated_file_meta_info: Vec<FileMetaInfo>,
    ) -> Self {
        Self::UpdatePeerInfo {
            peer_id,
            updated_file_meta_info,
        }
    }
}
/// keep_alive manager to peer info manager
pub enum KAToPIMessage {
    PeerOnlineMessage {
        peer_id: u32,
        peer_ip: Ipv4Addr,
    },
    PeerDroppedMessage {
        peer_id: u32,
    }
}

impl KAToPIMessage {

    pub fn new_peer_online_message(peer_id: u32, peer_ip: Ipv4Addr) -> Self {
        Self::PeerOnlineMessage {
            peer_id,
            peer_ip,
        }
    }

    pub fn new_peer_dropped_messgae(peer_id: u32) -> Self {
        Self::PeerDroppedMessage {
            peer_id,
        }
    }


}
/// p2p manager to peer info manager
pub struct P2PToPIMessage {}
