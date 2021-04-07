use crate::types::{FileMetaInfo, FilePieceInfo, PeerInfo, PeersInfoTable};
use bendy::decoding::{FromBencode, Object, ResultExt};
use bendy::encoding::{Error, SingleItemEncoder, ToBencode};
use std::collections::HashMap;
use std::net::IpAddr;
/// [ Message typeID <1 bytes> ][ Payload length <4 bytes> ][ Payload ]
/// 0 => `Peer`与`Tracker`之间保持联系的心跳包
/// 1 => `Peer`向`Tracker`注册信息的包
/// 2 => `Tracker`向`Peer`发送新加入的PeerInfo信息
/// 3 => `Tracker`向`Peer`发送完整的PeersInfoTable
/// 4 => `Tracker`向`Peer`发送用户更改文件信息用户添加文件信息
/// 5 => `Tracker`向`Peer`发送用户掉线信息
/// 6 => `Peer`向`Peer`发送文件请求的包
/// 7 => `Peer`向`Peer`发送文件块的包
/// 8 => `Tracker`向`Peer`发送用户下载块的信息

/// [ Message typeID <1> ]
#[derive(Debug, Eq, PartialEq)]
pub struct RegisterPayload {
    // 发送本地共享文件信息
    file_meta_info_report: Vec<FileMetaInfo>,
    // 用于共享文件的 端口号
    file_share_port: u16,
    // 用于分发PeersInfo信息的端口号
    peer_info_sync_open_port: u16,
}
impl RegisterPayload {
    pub fn new(
        file_meta_info_report: Vec<FileMetaInfo>,
        file_share_port: u16,
        peer_info_sync_open_port: u16,
    ) -> Self {
        Self {
            file_share_port,
            file_meta_info_report,
            peer_info_sync_open_port,
        }
    }

    pub fn get_file_meta_info_report(&self) -> Vec<FileMetaInfo> {
        self.file_meta_info_report
            .iter()
            .map(|f| f.clone())
            .collect()
    }

    pub fn get_file_share_port(&self) -> u16 {
        self.file_share_port
    }

    pub fn get_peer_info_sync_port(&self) -> u16 {
        self.peer_info_sync_open_port
    }
}

/// [ Message typeID <2> ]
/// 2 => `Tracker`向`Peer`发送新加入的PeerInfo信息
pub struct PeerInfoAddPayload {
    peer_info: PeerInfo,
}

impl PeerInfoAddPayload {
    pub fn new(peer_info: PeerInfo) -> Self {
        Self { peer_info }
    }

    pub fn get_inner_peer_info(self) -> PeerInfo {
        self.peer_info
    }
}

/// [ Message typeID <3> ]
#[derive(Debug, Eq, PartialEq)]
pub struct PeersInfoSetPayload {
    peer_id: u32,
    peer_ip: Vec<u8>,
    peers_info_table: PeersInfoTable,
    keep_alive_interval: u64,
    keep_alive_manager_open_port: u16,
}

impl PeersInfoSetPayload {
    pub fn new(
        peer_id: u32,
        peer_ip: Vec<u8>,
        peers_info_table: PeersInfoTable,
        keep_alive_interval: u64,
        keep_alive_manager_open_port: u16,
    ) -> Self {
        Self {
            peer_id,
            peer_ip,
            peers_info_table,
            keep_alive_interval,
            keep_alive_manager_open_port,
        }
    }

    pub fn get_peer_id(&self) -> u32 {
        self.peer_id
    }

    pub fn get_peer_ip(&self) -> Vec<u8> {
        self.peer_ip.clone()
    }

    pub fn get_keep_alive_interval(&self) -> u64 {
        self.keep_alive_interval
    }

    pub fn get_peers_info_table(&self) -> PeersInfoTable {
        self.peers_info_table.clone()
    }

    pub fn get_keep_alive_manager_open_port(&self) -> u16 {
        self.keep_alive_manager_open_port
    }
}
/// [ Message typeID <4> ]
#[derive(Debug)]
pub struct PeersInfoUpdatePayload {
    peer_id: u32,
    updated_file_meta_info: Vec<FileMetaInfo>,
}

impl PeersInfoUpdatePayload {
    pub fn new(peer_id: u32, updated_file_meta_info: Vec<FileMetaInfo>) -> Self {
        Self {
            peer_id,
            updated_file_meta_info,
        }
    }

    pub fn get_peer_id(&self) -> u32 {
        self.peer_id
    }

    pub fn get_updated_file_meta_info(&self) -> Vec<FileMetaInfo> {
        self.updated_file_meta_info.clone()
    }
}

/// [ Message typeID <5> ]
/// 用户掉线信息包
pub struct PeerDroppedPayload {
    peer_id: u32,
}

impl PeerDroppedPayload {
    pub fn new(peer_id: u32) -> Self {
        Self { peer_id }
    }

    pub fn get_dropped_peer_id(&self) -> u32 {
        self.peer_id
    }
}

///  [ Message typeID <6> ]
/// 6 => `Peer`向`Peer`发送文件请求的包
pub struct FilePieceRequestPayload {
    // 文件的sha3_code
    file_sha3_code: Vec<u8>,
    // piece_sha3_code
    file_piece_sha3_code: Vec<u8>,
    // piece_index
    file_piece_index: u32,
    // file_piece_size
    file_piece_size: u32,
}

impl FilePieceRequestPayload {
    pub fn new(
        file_sha3_code: Vec<u8>,
        file_piece_sha3_code: Vec<u8>,
        file_piece_index: u32,
        file_piece_size: u32,
    ) -> Self {
        Self {
            file_sha3_code,
            file_piece_sha3_code,
            file_piece_index,
            file_piece_size,
        }
    }

    pub fn get_inner_info(self) -> (Vec<u8>, Vec<u8>, u32, u32) {
        (
            self.file_sha3_code,
            self.file_piece_sha3_code,
            self.file_piece_index,
            self.file_piece_size,
        )
    }
}

///  [ Message typeID <8> ]
/// 8 => `Tracker`向`Peer`发送用户下载块的信息
pub struct FilePieceInfoAddPayload {
    peer_id: u32,
    file_piece_info: FilePieceInfo,
    file_meta_info_with_empty_piece: FileMetaInfo,
}

impl FilePieceInfoAddPayload {
    pub fn new(
        peer_id: u32,
        file_piece_info: FilePieceInfo,
        file_meta_info_with_empty_piece: FileMetaInfo,
    ) -> Self {
        Self {
            peer_id,
            file_piece_info,
            file_meta_info_with_empty_piece,
        }
    }

    pub fn get_inner_info(self) -> (u32, FilePieceInfo, FileMetaInfo) {
        (
            self.peer_id,
            self.file_piece_info,
            self.file_meta_info_with_empty_piece,
        )
    }
}

impl ToBencode for PeerInfoAddPayload {
    const MAX_DEPTH: usize = 10;

    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), bendy::encoding::Error> {
        encoder.emit_dict(|mut e| e.emit_pair(b"peer_info", &self.peer_info))?;
        Ok(())
    }
}

impl FromBencode for PeerInfoAddPayload {
    fn decode_bencode_object(object: Object) -> Result<Self, bendy::decoding::Error> {
        let mut peer_info = None;

        let mut dict = object.try_into_dictionary()?;
        while let Some(pair) = dict.next_pair()? {
            match pair {
                (b"peer_info", value) => {
                    peer_info = PeerInfo::decode_bencode_object(value)
                        .context("peer_info")
                        .map(Some)?;
                }
                (unknown_field, _) => {
                    return Err(bendy::decoding::Error::unexpected_field(
                        String::from_utf8_lossy(unknown_field),
                    ));
                }
            }
        }
        let peer_info =
            peer_info.ok_or_else(|| bendy::decoding::Error::missing_field("peer_info"))?;
        Ok(PeerInfoAddPayload::new(peer_info))
    }
}

impl ToBencode for FilePieceInfoAddPayload {
    const MAX_DEPTH: usize = 10;

    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), bendy::encoding::Error> {
        encoder.emit_unsorted_dict(|mut e| {
            e.emit_pair(b"peer_id", &self.peer_id)?;
            e.emit_pair(b"file_piece_info", &self.file_piece_info)?;
            e.emit_pair(
                b"file_meta_info_with_empty_piece",
                &self.file_meta_info_with_empty_piece,
            )
        })?;
        Ok(())
    }
}

impl FromBencode for FilePieceInfoAddPayload {
    fn decode_bencode_object(object: Object) -> Result<Self, bendy::decoding::Error> {
        let mut peer_id = None;
        let mut file_piece_info = None;
        let mut file_meta_info_with_empty_piece = None;

        let mut dict = object.try_into_dictionary()?;

        while let Some(pair) = dict.next_pair()? {
            match pair {
                (b"peer_id", value) => {
                    peer_id = u32::decode_bencode_object(value)
                        .context("peer_id")
                        .map(Some)?;
                }
                (b"file_piece_info", value) => {
                    file_piece_info = FilePieceInfo::decode_bencode_object(value)
                        .context("file_piece_info")
                        .map(Some)?;
                }
                (b"file_meta_info_with_empty_piece", value) => {
                    file_meta_info_with_empty_piece = FileMetaInfo::decode_bencode_object(value)
                        .context("file_meta_info_with_empty_piece")
                        .map(Some)?;
                }
                (unknown_field, _) => {
                    return Err(bendy::decoding::Error::unexpected_field(
                        String::from_utf8_lossy(unknown_field),
                    ));
                }
            }
        }

        let peer_id = peer_id.ok_or_else(|| bendy::decoding::Error::missing_field("peer_id"))?;
        let file_piece_info = file_piece_info
            .ok_or_else(|| bendy::decoding::Error::missing_field("file_piece_info"))?;
        let file_meta_info_with_empty_piece = file_meta_info_with_empty_piece.ok_or_else(|| {
            bendy::decoding::Error::missing_field("file_meta_info_with_empty_piece")
        })?;
        Ok(FilePieceInfoAddPayload::new(
            peer_id,
            file_piece_info,
            file_meta_info_with_empty_piece,
        ))
    }
}

impl ToBencode for FilePieceRequestPayload {
    const MAX_DEPTH: usize = 5;

    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), bendy::encoding::Error> {
        encoder.emit_unsorted_dict(|mut e| {
            e.emit_pair(b"file_sha3_code", &self.file_sha3_code)?;
            e.emit_pair(b"file_piece_sha3_code", &self.file_piece_sha3_code)?;
            e.emit_pair(b"file_piece_index", &self.file_piece_index)?;
            e.emit_pair(b"file_piece_size", &self.file_piece_size)
        })?;
        Ok(())
    }
}

impl FromBencode for FilePieceRequestPayload {
    fn decode_bencode_object(object: Object) -> Result<Self, bendy::decoding::Error> {
        let mut file_sha3_code = None;
        let mut file_piece_sha3_code = None;
        let mut file_piece_index = None;
        let mut file_piece_size = None;

        let mut dict = object.try_into_dictionary()?;
        while let Some(pair) = dict.next_pair()? {
            match pair {
                (b"file_sha3_code", value) => {
                    file_sha3_code = Vec::<u8>::decode_bencode_object(value)
                        .context("file_sha3_code")
                        .map(Some)?;
                }
                (b"file_piece_sha3_code", value) => {
                    file_piece_sha3_code = Vec::<u8>::decode_bencode_object(value)
                        .context("file_piece_sha3_code")
                        .map(Some)?;
                }
                (b"file_piece_index", value) => {
                    file_piece_index = u32::decode_bencode_object(value)
                        .context("file_piece_index")
                        .map(Some)?;
                }
                (b"file_piece_size", value) => {
                    file_piece_size = u32::decode_bencode_object(value)
                        .context("file_piece_size")
                        .map(Some)?;
                }
                (unknown_field, _) => {
                    return Err(bendy::decoding::Error::unexpected_field(
                        String::from_utf8_lossy(unknown_field),
                    ));
                }
            }
        }
        let file_sha3_code = file_sha3_code
            .ok_or_else(|| bendy::decoding::Error::missing_field("file_sha3_code"))?;
        let file_piece_sha3_code = file_piece_sha3_code
            .ok_or_else(|| bendy::decoding::Error::missing_field("file_piece_sha3_code"))?;
        let file_piece_index = file_piece_index
            .ok_or_else(|| bendy::decoding::Error::missing_field("file_piece_index"))?;
        let file_piece_size = file_piece_size
            .ok_or_else(|| bendy::decoding::Error::missing_field("file_piece_size"))?;

        Ok(FilePieceRequestPayload::new(
            file_sha3_code,
            file_piece_sha3_code,
            file_piece_index,
            file_piece_size,
        ))
    }
}

impl ToBencode for PeerDroppedPayload {
    const MAX_DEPTH: usize = 5;

    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), bendy::encoding::Error> {
        encoder.emit_dict(|mut e| e.emit_pair(b"peer_id", self.peer_id))?;
        Ok(())
    }
}

impl FromBencode for PeerDroppedPayload {
    fn decode_bencode_object(object: Object) -> Result<Self, bendy::decoding::Error> {
        let mut peer_id = None;

        let mut dict = object.try_into_dictionary()?;
        while let Some(pair) = dict.next_pair()? {
            match pair {
                (b"peer_id", value) => {
                    peer_id = u32::decode_bencode_object(value)
                        .context("peer_id")
                        .map(Some)?;
                }
                (unknown_field, _) => {
                    return Err(bendy::decoding::Error::unexpected_field(
                        String::from_utf8_lossy(unknown_field),
                    ));
                }
            }
        }

        let peer_id = peer_id.ok_or_else(|| bendy::decoding::Error::missing_field("peer_id"))?;
        Ok(PeerDroppedPayload::new(peer_id))
    }
}

impl ToBencode for PeersInfoUpdatePayload {
    const MAX_DEPTH: usize = 10;
    //   peer_id: u32,
    //   updated_file_meta_info: Vec<FileMetaInfo>,
    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), bendy::encoding::Error> {
        encoder.emit_dict(|mut e| {
            e.emit_pair(b"peer_id", &self.peer_id)?;
            e.emit_pair(b"updated_file_meta_info", &self.updated_file_meta_info)
        })?;
        Ok(())
    }
}

impl FromBencode for PeersInfoUpdatePayload {
    fn decode_bencode_object(object: Object) -> Result<Self, bendy::decoding::Error> {
        let mut peer_id = None;
        let mut updated_file_meta_info = None;

        let mut dict = object.try_into_dictionary()?;
        while let Some(pair) = dict.next_pair()? {
            match pair {
                (b"peer_id", value) => {
                    peer_id = u32::decode_bencode_object(value)
                        .context("peer_id")
                        .map(Some)?;
                }
                (b"updated_file_meta_info", value) => {
                    updated_file_meta_info = Vec::<FileMetaInfo>::decode_bencode_object(value)
                        .context("updated_file_meta_info")
                        .map(Some)?;
                }
                (unknown_field, _) => {
                    return Err(bendy::decoding::Error::unexpected_field(
                        String::from_utf8_lossy(unknown_field),
                    ));
                }
            }
        }
        let peer_id = peer_id.ok_or_else(|| bendy::decoding::Error::missing_field("peer_id"))?;
        let updated_file_meta_info = updated_file_meta_info
            .ok_or_else(|| bendy::decoding::Error::missing_field("updated_file_meta_info"))?;

        Ok(PeersInfoUpdatePayload::new(peer_id, updated_file_meta_info))
    }
}

impl ToBencode for PeersInfoSetPayload {
    const MAX_DEPTH: usize = 10;
    //     peer_id: u32,
    //     peer_ip: Vec<u8>,
    //     peers_info_table: PeersInfoTable,
    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), bendy::encoding::Error> {
        encoder.emit_unsorted_dict(|mut e| {
            e.emit_pair(b"peer_id", self.peer_id)?;
            e.emit_pair(b"peer_ip", &self.peer_ip)?;
            e.emit_pair(b"peers_info_table", &self.peers_info_table)?;
            e.emit_pair(b"keep_alive_interval", self.keep_alive_interval)?;
            e.emit_pair(
                b"keep_alive_manager_open_port",
                &self.keep_alive_manager_open_port,
            )
        })?;
        Ok(())
    }
}

impl FromBencode for PeersInfoSetPayload {
    fn decode_bencode_object(object: Object) -> Result<Self, bendy::decoding::Error> {
        let mut peer_id = None;
        let mut peer_ip = None;
        let mut peers_info_table = None;
        let mut keep_alive_interval = None;
        let mut keep_alive_manager_open_port = None;

        let mut dict = object.try_into_dictionary()?;
        while let Some(pair) = dict.next_pair()? {
            match pair {
                (b"peer_id", value) => {
                    peer_id = u32::decode_bencode_object(value)
                        .context("peer_id")
                        .map(Some)?;
                }
                (b"peer_ip", value) => {
                    peer_ip = Vec::<u8>::decode_bencode_object(value)
                        .context("peer_ip")
                        .map(Some)?;
                }
                (b"peers_info_table", value) => {
                    peers_info_table = PeersInfoTable::decode_bencode_object(value)
                        .context("peers_info_table")
                        .map(Some)?;
                }
                (b"keep_alive_interval", value) => {
                    keep_alive_interval = u64::decode_bencode_object(value)
                        .context("keep_alive_interval")
                        .map(Some)?;
                }
                (b"keep_alive_manager_open_port", value) => {
                    keep_alive_manager_open_port = u16::decode_bencode_object(value)
                        .context("keep_alive_manager_open_port")
                        .map(Some)?;
                }
                (unknown_field, _) => {
                    return Err(bendy::decoding::Error::unexpected_field(
                        String::from_utf8_lossy(unknown_field),
                    ));
                }
            }
        }
        let peer_id = peer_id.ok_or_else(|| bendy::decoding::Error::missing_field("peer_id"))?;
        let peer_ip = peer_ip.ok_or_else(|| bendy::decoding::Error::missing_field("peer_ip"))?;
        let peers_info_table = peers_info_table
            .ok_or_else(|| bendy::decoding::Error::missing_field("peers_info_table"))?;
        let keep_alive_interval = keep_alive_interval
            .ok_or_else(|| bendy::decoding::Error::missing_field("keep_alive_interval"))?;
        let keep_alive_manager_open_port = keep_alive_manager_open_port
            .ok_or_else(|| bendy::decoding::Error::missing_field("keep_alive_manager_open_port"))?;
        Ok(PeersInfoSetPayload::new(
            peer_id,
            peer_ip,
            peers_info_table,
            keep_alive_interval,
            keep_alive_manager_open_port,
        ))
    }
}

impl ToBencode for RegisterPayload {
    const MAX_DEPTH: usize = 7;
    /// struct RegisterPayload {
    //     // 发送本地共享文件信息
    //     file_meta_info_report: Vec<FileMetaInfo>,
    //     // 用于共享文件的 端口号
    //     file_share_port: u16,
    // }
    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), bendy::encoding::Error> {
        encoder.emit_dict(|mut e| {
            e.emit_pair(b"file_meta_info_report", &self.file_meta_info_report)?;
            e.emit_pair(b"file_share_port", &self.file_share_port)?;
            e.emit_pair(b"peer_info_sync_open_port", &self.peer_info_sync_open_port)
        })?;
        Ok(())
    }
}

impl FromBencode for RegisterPayload {
    /// struct RegisterPayload {
    //     // 发送本地共享文件信息
    //     file_meta_info_report: Vec<FileMetaInfo>,
    //     // 用于共享文件的 端口号
    //     file_share_port: u16,
    // }
    fn decode_bencode_object(object: Object) -> Result<Self, bendy::decoding::Error> {
        let mut file_meta_info_report = None;
        let mut file_share_port = None;
        let mut peer_info_sync_open_port = None;
        let mut dict = object.try_into_dictionary()?;
        while let Some(pair) = dict.next_pair()? {
            match pair {
                (b"file_meta_info_report", value) => {
                    file_meta_info_report = Vec::<FileMetaInfo>::decode_bencode_object(value)
                        .context("file_meta_info_report")
                        .map(Some)?;
                }
                (b"file_share_port", value) => {
                    file_share_port = u16::decode_bencode_object(value)
                        .context("file_share_port")
                        .map(Some)?;
                }
                (b"peer_info_sync_open_port", value) => {
                    peer_info_sync_open_port = u16::decode_bencode_object(value)
                        .context("peer_info_sync_open_port")
                        .map(Some)?;
                }
                (unknown_field, _) => {
                    return Err(bendy::decoding::Error::unexpected_field(
                        String::from_utf8_lossy(unknown_field),
                    ));
                }
            }
        }
        let file_meta_info_report = file_meta_info_report
            .ok_or_else(|| bendy::decoding::Error::missing_field("file_meta_info_report"))?;
        let file_share_port = file_share_port
            .ok_or_else(|| bendy::decoding::Error::missing_field("file_share_port"))?;
        let peer_info_sync_open_port = peer_info_sync_open_port
            .ok_or_else(|| bendy::decoding::Error::missing_field("peer_info_sync_open_port"))?;
        Ok(RegisterPayload {
            file_meta_info_report,
            file_share_port,
            peer_info_sync_open_port,
        })
    }
}
