use crate::types::{FileMetaInfo, PeerInfo};
use bendy::decoding::{FromBencode, Object, ResultExt};
use bendy::encoding::{Error, SingleItemEncoder, ToBencode};
use std::collections::HashMap;
use std::net::IpAddr;

/// 0 => 心跳包
/// 1 => Peer 向 Tracker注册信息的包
/// 2 => Tracker 向 Peer 回复的包
/// 3 => Peer Meta Info table Update 数据包(add new file)
/// 4 => Peer Request to Peer
/// 5 => Peer Response to Peer

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
    pub fn new(file_meta_info_report: Vec<FileMetaInfo>, file_share_port: u16, peer_info_sync_open_port: u16) -> Self {
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

#[derive(Debug, Eq, PartialEq)]
pub struct RegisterResponsePayload {
    // peer_id 用于标识一个对等方，Tracker的peer_id为 0
    peer_id: u32,
    // peer_ip 告诉对等方IP地址(: TODO 暂时只支持IPv4)
    peer_ip: Vec<u8>,
    // 对等方信息
    peers_info: Vec<PeerInfo>,
    // 保持通信的间隔(ms)
    interval: u32,
}
#[derive(Debug, Eq, PartialEq)]
pub struct PeersInfoUpdatePayload {
    // 0 => set
    // 1 => update
    cmd_code: u8,
    //

}

impl PeersInfoUpdatePayload {
    pub fn new(cmd_code: u8, ) -> Self {
        Self {
            cmd_code,

        }
    }
}

impl RegisterResponsePayload {
    pub fn new(peer_id: u32, peer_ip: Vec<u8>, peers_info: Vec<PeerInfo>, interval: u32) -> Self {
        Self {
            peer_id,
            peer_ip,
            peers_info,
            interval,
        }
    }
    pub fn get_other_info(&self) -> (u32, u32, [u8; 4]) {
        assert_eq!(self.peer_ip.len(), 4);
        let ip_addr = [
            self.peer_ip[0],
            self.peer_ip[1],
            self.peer_ip[2],
            self.peer_ip[3],
        ];
        (self.peer_id, self.interval, ip_addr)
    }
    pub fn get_peers_info(&self) -> Vec<PeerInfo> {
        self.peers_info.iter().map(|p| p.clone()).collect()
    }
}

impl ToBencode for RegisterResponsePayload {
    const MAX_DEPTH: usize = 7;
    /// pub struct RegisterResponsePayload {
    //     // peer_id 用于标识一个对等方，Tracker的peer_id为 0
    //     peer_id: u32,
    //     // peer_ip 告诉对等方IP地址
    //     peer_ip: Vec<u8>,
    //     // 对等方信息
    //     peers_info: Vec<PeerInfo>,
    //     // 保持通信的间隔(ms)
    //     interval: u32,
    // }
    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), bendy::encoding::Error> {
        encoder.emit_dict(|mut e| {
            e.emit_pair(b"peer_id", self.peer_id)?;
            e.emit_pair(b"peer_ip", &self.peer_ip)?;
            e.emit_pair(b"interval", self.interval)?;
            e.emit_pair(b"peers_info", &self.peers_info)
        })?;
        Ok(())
    }
}

impl FromBencode for RegisterResponsePayload {
    /// pub struct RegisterResponsePayload {
    //     // peer_id 用于标识一个对等方，Tracker的peer_id为 0
    //     peer_id: u32,
    //     // peer_ip 告诉对等方IP地址
    //     peer_ip: Vec<u8>,
    //     // 对等方信息
    //     peers_info: Vec<PeerInfo>,
    //     // 保持通信的间隔(ms)
    //     interval: u32,
    // }
    fn decode_bencode_object(object: Object) -> Result<Self, bendy::decoding::Error> {
        let mut peer_id = None;
        let mut peer_ip = None;
        let mut peers_info = None;
        let mut interval = None;

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
                (b"peers_info", value) => {
                    peers_info = Vec::<PeerInfo>::decode_bencode_object(value)
                        .context("peers_info")
                        .map(Some)?;
                }
                (b"interval", value) => {
                    interval = u32::decode_bencode_object(value)
                        .context("interval")
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
        let peers_info =
            peers_info.ok_or_else(|| bendy::decoding::Error::missing_field("peers_info"))?;
        let interval = interval.ok_or_else(|| bendy::decoding::Error::missing_field("interval"))?;
        Ok(RegisterResponsePayload::new(
            peer_id, peer_ip, peers_info, interval,
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
                },
                (b"file_share_port", value) => {
                    file_share_port = u16::decode_bencode_object(value)
                        .context("file_share_port")
                        .map(Some)?;
                },
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
        let peer_info_sync_open_port = peer_info_sync_open_port.ok_or_else(|| bendy::decoding::Error::missing_field("peer_info_sync_open_port"))?;
        Ok(RegisterPayload {
            file_meta_info_report,
            file_share_port,
            peer_info_sync_open_port,
        })
    }
}
