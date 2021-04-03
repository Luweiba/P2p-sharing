use crate::message::{PeersInfoUpdatePayload, RegisterPayload, RegisterResponsePayload};
use crate::thread_communication_message::{P2TToPIMessage, T2PToPIMessage};
use crate::types::{FileMetaInfo, PeerInfo};
use bendy::decoding::FromBencode;
use bendy::encoding::ToBencode;
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
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
    // tracker IP
    tracker_ip: Ipv4Addr,
    // tracker port
    tracker_port: u16,
    // 局域网内IP
    routable_ip: Option<Ipv4Addr>,
}

impl PeerToTrackerManager {
    pub fn new(tracker_ip: Ipv4Addr, tracker_port: u16) -> Self {
        Self {
            peer_id: None,
            tracker_ip,
            tracker_port,
            routable_ip: None,
        }
    }

    pub async fn register_to_tracker(
        &mut self,
        file_meta_info_report: Vec<FileMetaInfo>,
        open_port: u16,
        peer_info_sync_open_port: u16,
        pi_sender: Sender<P2TToPIMessage>,
        mut pi_receiver: Receiver<P2TToPIMessage>,
    ) -> Result<(), Box<dyn Error>> {
        let mut client_stream = TcpStream::connect(SocketAddr::from((
            self.tracker_ip.octets(),
            self.tracker_port,
        )))
        .await?;
        let mut register_packet = Vec::<u8>::new();
        // 发送注册包
        register_packet.push(1u8);
        let register_payload =
            RegisterPayload::new(file_meta_info_report, open_port, peer_info_sync_open_port);
        let payload_encoded = register_payload.to_bencode().unwrap();
        let payload_len = payload_encoded.len() as u32;
        // little endian
        register_packet.extend_from_slice(&payload_len.to_le_bytes());
        register_packet.extend(payload_encoded.into_iter());
        client_stream.write(register_packet.as_slice()).await?;
        // 处理pi_receiver
        tokio::spawn(async move {
            println!("Start a thread to send local file update info to Tracker");
            loop {
                while let Some(message) = pi_receiver.recv().await {
                    println!("Get a Message from PI manager");
                    match message {
                        P2TToPIMessage::UpdatePeerInfo {
                            peer_id,
                            updated_file_meta_info,
                        } => {
                            let mut packet = Vec::<u8>::new();
                            packet.push(4u8);
                            let payload =
                                PeersInfoUpdatePayload::new(peer_id, updated_file_meta_info)
                                    .to_bencode()
                                    .unwrap();
                            let payload_length = payload.len() as u32;
                            packet.extend_from_slice(&payload_length.to_le_bytes().to_vec());
                            packet.extend_from_slice(&payload);
                            client_stream.write(&packet).await;
                        }
                        _ => {}
                    }
                }
            }
        });
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

    pub async fn start(
        &self,
        pi_sender: Sender<T2PToPIMessage>,
        mut pi_receiver: Receiver<T2PToPIMessage>,
    ) -> Result<(), Box<dyn Error>> {
        let tracker_listener = TcpListener::bind(SocketAddr::from((
            self.tracker_ip.octets(),
            self.tracker_port,
        )))
        .await?;
        println!(
            "Tracker Listening on {:?}",
            SocketAddr::from((self.tracker_ip.octets(), self.tracker_port,))
        );
        let mut peers_ip = Arc::new(Mutex::new(Vec::<SocketAddr>::new()));
        let peers_ip_clone = peers_ip.clone();
        // 处理来自PeersInfoManager的信息
        println!("Handle message from PeersInfoManagerPeer ...");
        tokio::spawn(async move {
            loop {
                if let Some(msg) = pi_receiver.recv().await {}
            }
        });
        println!("Ready to handle connection from Peer");
        loop {
            let (mut stream, peer_addr) = tracker_listener.accept().await?;
            println!("got a connection from {:?}", peer_addr);
            let peer_id_allocated_mutex = self.peer_id_allocated_begin_with.clone();
            let pi_sender_clone = pi_sender.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; P2T_BUF_SIZE];
                // 获取对等方IP地址
                let peer_ip_port = stream.peer_addr().unwrap().port();
                // 只支持IPv4
                if let IpAddr::V4(peer_ip) = stream.peer_addr().unwrap().ip() {
                    while let Ok(nbytes) = stream.read(&mut buf[..]).await {
                        if nbytes < 5 {
                            continue;
                        }
                        let type_id = buf[0];
                        let payload_bytes = [buf[1], buf[2], buf[3], buf[4]];
                        let payload_length = u32::from_le_bytes(payload_bytes);
                        assert_eq!(payload_length, (nbytes - 5) as u32);
                        match type_id {
                            // Register 包
                            //     // 发送本地共享文件信息
                            //     file_meta_info_report: Vec<FileMetaInfo>,
                            //     // 用于共享文件的 端口号
                            //     file_share_port: u16,
                            1u8 => {
                                if let Ok(register) = RegisterPayload::from_bencode(&buf[5..nbytes])
                                {
                                    println!("get Register Packet");
                                    let file_meta_info_report =
                                        register.get_file_meta_info_report();
                                    let peer_open_port = register.get_file_share_port();
                                    let peer_info_sync_open_port =
                                        register.get_peer_info_sync_port();
                                    // TODO
                                    // 更新PeersInfoTable
                                    let peer_id;
                                    {
                                        let mut peer_id_lock = peer_id_allocated_mutex.lock().await;
                                        peer_id = peer_id_lock.clone();
                                        peer_id_lock.checked_add(1);
                                    }
                                    let peer_info = PeerInfo::new(
                                        peer_id.clone(),
                                        peer_ip.octets().to_vec(),
                                        peer_open_port,
                                        file_meta_info_report,
                                    );
                                    // TODO !!!
                                    println!("send msg to PeerInfoManager");
                                    pi_sender_clone
                                        .send(T2PToPIMessage::new_add_one_peer_info(
                                            peer_info,
                                            peer_info_sync_open_port,
                                        ))
                                        .await;
                                }
                            },
                            4u8 => {
                                // 收到一个peer的本地更新信息
                                println!("get an update info from a Peer");
                                if let Ok(peers_info_update_payload) =
                                    PeersInfoUpdatePayload::from_bencode(&buf[5..nbytes])
                                {
                                    let peer_id = peers_info_update_payload.get_peer_id();
                                    let updated_file_meta_info =
                                        peers_info_update_payload.get_updated_file_meta_info();
                                    pi_sender_clone
                                        .send(T2PToPIMessage::new_update_one_peer_info(
                                            peer_id,
                                            updated_file_meta_info,
                                        ))
                                        .await;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            });
        }
        Ok(())
    }
}
