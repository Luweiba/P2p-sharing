use crate::message::{RegisterPayload, RegisterResponsePayload};
use crate::thread_communication_message::P2TToPIMessage;
use crate::types::{FileMetaInfo, PeerInfo};
use bendy::decoding::FromBencode;
use bendy::encoding::ToBencode;
use std::error::Error;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;
use uuid::Builder;

const P2T_BUF_SIZE: usize = 1 << 18;
/// 负责管理Peer与Tracker的通信
/// 主要包括：注册、更新、保持联系
pub struct PeerToTrackerManager {
    // peer_id 用于标识一个对等方，Tracker的peer_id为 0
    peer_id: Option<u32>,
    // 本机用于Peer to Tracker的TCPStream
    // p2t_client_stream: TcpStream,
    // tracker IP
    tracker_ip: Ipv4Addr,
    // tracker port
    tracker_port: u16,
    // 局域网内IP
    routable_ip: Option<Ipv4Addr>,
    // 保持通信的间隔(ms)
    interval: Option<u32>,
}

impl PeerToTrackerManager {
    pub fn new(tracker_ip: Ipv4Addr, tracker_port: u16) -> Self {
        Self {
            peer_id: None,
            tracker_ip,
            tracker_port,
            routable_ip: None,
            interval: None,
        }
    }

    pub async fn register_to_tracker(
        &mut self,
        file_meta_info_report: Vec<FileMetaInfo>,
        open_port: u16,
        pi_sender: Sender<P2TToPIMessage>,
        pi_receiver: Receiver<P2TToPIMessage>,
    ) -> Result<(), Box<dyn Error>> {
        let mut client_stream = TcpStream::connect(SocketAddr::from((
            self.tracker_ip.octets(),
            self.tracker_port,
        )))
        .await?;
        let mut register_packet = Vec::<u8>::new();
        // 发送注册包
        // Set message type
        register_packet.push(1u8);
        let register_payload = RegisterPayload::new(file_meta_info_report, open_port);
        let payload_encoded = register_payload.to_bencode().unwrap();
        let payload_len = payload_encoded.len() as u32;
        // little endian
        register_packet.extend_from_slice(&payload_len.to_le_bytes());
        register_packet.extend(payload_encoded.into_iter());
        println!("Send len: {}", register_packet.len());
        //println!("Send Packet \n {}", buf.iter().skip(5).map(|s| char::from(*s)).collect::<String>());
        client_stream.write(register_packet.as_slice()).await?;
        println!("Connected Tracker");
        // (peer_id, interval, ip_addr)
        let (tx, rx) = mpsc::channel::<(u32, u32, [u8; 4])>(5);
        let mut buf = vec![0u8; P2T_BUF_SIZE];
        tokio::spawn(async move {
            // handle data from tracker
            // TODO
            while let Ok(nbytes) = client_stream.read(&mut buf[..]).await {
                if nbytes < 5 {
                    continue;
                }
                let type_id = buf[0];
                let payload_bytes = [buf[1], buf[2], buf[3], buf[4]];
                let payload_length = u32::from_le_bytes(payload_bytes);
                assert_eq!(payload_length, (nbytes - 5) as u32);
                match type_id {
                    // 心跳包
                    0 => {
                        buf[0] = 0;
                        client_stream.write(&buf[..5]).await.unwrap();
                    }
                    // RegisterResponse 回复包
                    2 => {
                        if let Ok(register_response) =
                            RegisterResponsePayload::from_bencode(&buf[5..nbytes])
                        {
                            tx.send(register_response.get_other_info()).await.unwrap();
                        } else {
                            continue;
                        }
                    }
                    _ => {}
                }
            }
        })
        .await;
        Ok(())
    }
}

pub struct TrackerToPeerManager {
    // peer_id_counter
    peer_id_allocated_begin_with: Arc<Mutex<u32>>,
    // peer_id 用于标识一个对等方，Tracker的peer_id为 0
    peer_id: u32,
    // 本机用于Peer to Tracker的TCPStream
    // t2p_tracker_listener: TcpListener,
    // tracker IP
    tracker_ip: Ipv4Addr,
    // tracker port
    tracker_port: u16,
    // 保持通信的间隔(ms)
    interval: u32,
}

impl TrackerToPeerManager {
    pub fn new(tracker_ip: Ipv4Addr, tracker_port: u16, interval: u32) -> Self {
        Self {
            peer_id_allocated_begin_with: Arc::new(Mutex::new(1)),
            peer_id: 0,
            tracker_ip,
            tracker_port,
            interval,
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn Error>> {
        let tracker_listener = TcpListener::bind(SocketAddr::from((
            self.tracker_ip.octets(),
            self.tracker_port,
        )))
        .await?;
        loop {
            let (mut stream, peer_addr) = tracker_listener.accept().await?;
            let peer_id_allocated_mutex = self.peer_id_allocated_begin_with.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; P2T_BUF_SIZE];
                while let Ok(nbytes) = stream.read(&mut buf[..]).await {
                    if nbytes < 5 {
                        continue;
                    }
                    let type_id = buf[0];
                    let payload_bytes = [buf[1], buf[2], buf[3], buf[4]];
                    let payload_length = u32::from_le_bytes(payload_bytes);
                    assert_eq!(payload_length, (nbytes - 5) as u32);
                    match type_id {
                        // heartbeat packet
                        0 => {
                            // TODO
                            // 更新计时，稍后再发送包
                            buf[0] = 0;
                            stream.write(&buf[..5]).await.unwrap();
                        }
                        // Register 包
                        //     // 发送本地共享文件信息
                        //     file_meta_info_report: Vec<FileMetaInfo>,
                        //     // 用于共享文件的 端口号
                        //     file_share_port: u16,
                        1 => {
                            if let Ok(register) = RegisterPayload::from_bencode(&buf[5..nbytes]) {
                                let file_meta_info_report = register.get_file_meta_info_report();
                                let file_share_port = register.get_file_share_port();
                                // TODO
                                // 更新PeersInfoTable

                                let peer_id = peer_id_allocated_mutex.lock().await;
                            } else {
                                continue;
                            }
                        }
                        _ => {}
                    }
                }
            })
            .await?;
        }
        Ok(())
    }
}
