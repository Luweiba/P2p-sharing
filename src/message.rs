use std::net::IpAddr;
use std::collections::HashMap;
use bendy::encoding::{ToBencode, Error, SingleItemEncoder};
use crate::types::{PeerInfo, FileMetaInfo};
use bendy::decoding::{FromBencode, Object, ResultExt};

#[derive(Debug, Eq, PartialEq)]
pub struct Message {
    /// 用于标识报文类型
    /// 0 => 心跳包
    /// 1 => Peer 向 Tracker注册信息的包
    /// 2 => Tracker 向 Peer 回复的包
    message_type: u8,
    // 有效载荷
    message_payload: Payload,
}

#[derive(Debug, Eq, PartialEq)]
enum Payload {
    // Message ID 0
    HeartBeat,
    // Message ID 1
    Register(RegisterPayload),
    // Message ID 2
    RegisterResponse(RegisterResponsePayload)
}

#[derive(Debug, Eq, PartialEq)]
struct RegisterPayload {
    // 发送本地共享文件信息
    file_meta_info_report: Vec<FileMetaInfo>,
    // 用于共享文件的 端口号
    file_share_port: u16,
}

#[derive(Debug, Eq, PartialEq)]
struct RegisterResponsePayload {
    // peer_id 用于标识一个对等方，Tracker的peer_id为 0
    peer_id: u32,
    // peer_ip 告诉对等方IP地址(: TODO 暂时只支持IPv4)
    peer_ip: Vec<u8>,
    // 对等方信息
    peers_info: Vec<PeerInfo>,
    // 保持通信的间隔(ms)
    interval: u32,
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
            e.emit_pair(b"file_share_port", &self.file_share_port)
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
               (unknown_field, _) => {
                   return Err(bendy::decoding::Error::unexpected_field(String::from_utf8_lossy(unknown_field)));
               }
           }
        }
        let file_meta_info_report = file_meta_info_report.ok_or_else(|| bendy::decoding::Error::missing_field("file_meta_info_report"))?;
        let file_share_port = file_share_port.ok_or_else(|| bendy::decoding::Error::missing_field("file_share_port"))?;
        Ok(RegisterPayload {
            file_meta_info_report,
            file_share_port
        })
    }
}