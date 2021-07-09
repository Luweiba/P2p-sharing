use crate::types::{FileMetaInfo, FilePieceInfo, PeerInfo, PeersInfoTable};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use tokio::net::TcpStream;
use tokio::sync::oneshot;

/// local file manager to peer info manager
/// 本地用于告知PeerInfoManager本地文件变动
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
    AddOnePeerInfo {
        peer_info: PeerInfo,
    },
    UpdatePeerInfo {
        peer_id: u32,
        updated_file_meta_info: Vec<FileMetaInfo>,
    },
    UpdatePieceDownloadedInfo {
        peer_id: u32,
        file_piece_info: FilePieceInfo,
        file_meta_info_with_empty_piece: FileMetaInfo,
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

    pub fn new_update_piece_downloaded_info(
        peer_id: u32,
        file_piece_info: FilePieceInfo,
        file_meta_info_with_empty_piece: FileMetaInfo,
    ) -> Self {
        Self::UpdatePieceDownloadedInfo {
            peer_id,
            file_piece_info,
            file_meta_info_with_empty_piece,
        }
    }
}
/// t2p manager to peer info manager
#[derive(Debug)]
pub enum T2PToPIMessage {
    AddOnePeerInfo {
        peer_info: PeerInfo,
        peer_info_sync_open_port: u16,
    },
    UpdatePeerInfo {
        peer_id: u32,
        updated_file_meta_info: Vec<FileMetaInfo>,
    },
    UpdatePieceDownloadedInfo {
        peer_id: u32,
        file_piece_info: FilePieceInfo,
        file_meta_info_with_empty_piece: FileMetaInfo,
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

    pub fn new_update_piece_downloaded_info(
        peer_id: u32,
        file_piece_info: FilePieceInfo,
        file_meta_info_with_empty_piece: FileMetaInfo,
    ) -> Self {
        Self::UpdatePieceDownloadedInfo {
            peer_id,
            file_piece_info,
            file_meta_info_with_empty_piece,
        }
    }
}
/// keep_alive manager to peer info manager
pub enum KAToPIMessage {
    PeerOnlineMessage { peer_id: u32, peer_ip: Ipv4Addr },
    PeerDroppedMessage { peer_id: u32 },
}

impl KAToPIMessage {
    pub fn new_peer_online_message(peer_id: u32, peer_ip: Ipv4Addr) -> Self {
        Self::PeerOnlineMessage { peer_id, peer_ip }
    }

    pub fn new_peer_dropped_message(peer_id: u32) -> Self {
        Self::PeerDroppedMessage { peer_id }
    }
}
/// p2p manager to peer info manager
pub enum P2PToPIMessage {
    GetPeersInfoTableSnapshot {
        sender: oneshot::Sender<PeersInfoTable>,
    },
    DownloadedFileInfo {
        file_download_sha3_code: Vec<u8>,
        peers_info_table_snapshot: PeersInfoTable,
        sender: oneshot::Sender<()>,
    },
    PieceDownloadedInfo {
        file_meta_info_with_empty_piece_info: FileMetaInfo,
        file_piece_info: FilePieceInfo,
    },
}

impl P2PToPIMessage {
    pub fn new_get_peers_info_table_snapshot_message(
        sender: oneshot::Sender<PeersInfoTable>,
    ) -> Self {
        Self::GetPeersInfoTableSnapshot { sender }
    }

    pub fn new_downloaded_file_info_message(
        file_download_sha3_code: Vec<u8>,
        peers_info_table_snapshot: PeersInfoTable,
        sender: oneshot::Sender<()>,
    ) -> Self {
        Self::DownloadedFileInfo {
            file_download_sha3_code,
            peers_info_table_snapshot,
            sender,
        }
    }
    pub fn new_piece_downloaded_info_message(
        file_meta_info_with_empty_piece_info: FileMetaInfo,
        file_piece_info: FilePieceInfo,
    ) -> Self {
        Self::PieceDownloadedInfo {
            file_meta_info_with_empty_piece_info,
            file_piece_info,
        }
    }
}

pub enum P2PToLFMessage {
    GetLocalPath {
        sender: oneshot::Sender<PathBuf>,
        file_sha3_code: Vec<u8>,
    },
    DownloadedFileStoreAndUpdate {
        // 用于告知文件存储成功
        sender: oneshot::Sender<tokio::fs::File>,
        file_meta_info: FileMetaInfo,
    },
}

impl P2PToLFMessage {
    pub fn new_get_local_path(file_sha3_code: Vec<u8>, sender: oneshot::Sender<PathBuf>) -> Self {
        Self::GetLocalPath {
            sender,
            file_sha3_code,
        }
    }

    pub fn new_downloaded_file_store_and_update(
        sender: oneshot::Sender<tokio::fs::File>,
        file_meta_info: FileMetaInfo,
    ) -> Self {
        Self::DownloadedFileStoreAndUpdate {
            // 给我一个可以异步写入的file
            sender,
            file_meta_info,
        }
    }
}
