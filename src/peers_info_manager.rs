use crate::thread_communication_message::{LFToPIMessage, P2PToPIMessage, P2TToPIMessage, T2PToPIMessage};
use crate::types::{PeerInfo, PeersInfoTable};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use uuid::Uuid;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use bendy::encoding::ToBencode;
use bendy::decoding::FromBencode;

const BUF_SIZE: usize = 1<<18;



pub struct PeersInfoManagerOfTracker {
    // 全局的Peers Info Table
    peers_info_table: Arc<Mutex<PeersInfoTable>>
}

impl PeersInfoManagerOfTracker {
    pub async fn start(
        &mut self,
        lf_sender: Sender<LFToPIMessage>,
        mut lf_receiver: Receiver<LFToPIMessage>,
        t2p_sender: Sender<T2PToPIMessage>,
        mut t2p_receiver: Receiver<T2PToPIMessage>,
        p2p_sender: Sender<P2PToPIMessage>,
        mut p2p_receiver: Receiver<P2PToPIMessage>,
    ) -> Result<(), Box<dyn Error>> {
        // TODO 处理通信逻辑
        // lf_receiver
        let lf_peers_info_table = self.peers_info_table.clone();
        tokio::spawn(async move {
            loop {
                if let Some(msg) = lf_receiver.recv().await {}
            }
        });
        // t2p_receiver
        let t2p_peers_info_table = self.peers_info_table.clone();
        tokio::spawn(async move {
            loop {
                if let Some(msg) = t2p_receiver.recv().await {
                    println!("Get a msg {:?}", msg);
                    match msg {
                        T2PToPIMessage::AddOnePeerInfo { peer_info, peer_info_sync_open_port } => {
                            let peer_local_bind_ip = peer_info.get_peer_ip();
                            let peers_info_table_snapshot;
                            let peer_ip = peer_info.get_peer_ip();
                            {
                                let mut peers_info_table = t2p_peers_info_table.lock().await;
                                peers_info_table.add_one_peer_info(peer_info);
                                peers_info_table_snapshot = peers_info_table.get_a_snapshot();
                            }
                            // 获取更新后的表的信息，发送给peer的客户端
                            tokio::spawn(async move {
                                let peer_local_bind_ip = [peer_local_bind_ip[0], peer_local_bind_ip[1], peer_local_bind_ip[2], peer_local_bind_ip[3]];
                                if let Ok(mut stream) = TcpStream::connect(SocketAddr::from(SocketAddrV4::new(Ipv4Addr::from(peer_local_bind_ip), peer_info_sync_open_port))).await {
                                    println!("Connected with {:?}", SocketAddr::from(SocketAddrV4::new(Ipv4Addr::from(peer_local_bind_ip), peer_info_sync_open_port)));
                                    // 发送peerinfo文件包
                                    let mut set_peers_info_table_packet = Vec::<u8>::new();
                                    // code 3 => set peers_info_table_packet
                                    set_peers_info_table_packet.push(3u8);
                                    let payload = peers_info_table_snapshot.to_bencode().unwrap();
                                    let payload_len = payload.len() as u32;
                                    set_peers_info_table_packet.extend_from_slice(&payload_len.to_le_bytes());
                                    set_peers_info_table_packet.extend_from_slice(&payload);
                                    // 发送包
                                    stream.write(&set_peers_info_table_packet).await.unwrap();
                                }
                            });
                        },
                        T2PToPIMessage::UpdatePeerInfo {} => {

                        }
                    }
                }
            }
        });
        // p2p_receiver
        let p2t_peers_info_table = self.peers_info_table.clone();
        tokio::spawn(async move {
            loop {
                if let Some(msg) = p2p_receiver.recv().await {}
            }
        });
        Ok(())
    }

    pub fn new(peers_info_table: PeersInfoTable) -> Self {
        Self {
            peers_info_table: Arc::new(Mutex::new(peers_info_table)),
        }
    }
}
/// 全局PeersInfo信息的去重与实时更新
pub struct PeersInfoManagerOfPeer {
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

impl PeersInfoManagerOfPeer {
    pub async fn start(
        &mut self,
        local_bind_ip: Ipv4Addr,
        peer_info_sync_open_port: u16,
        lf_sender: Sender<LFToPIMessage>,
        mut lf_receiver: Receiver<LFToPIMessage>,
        p2t_sender: Sender<P2TToPIMessage>,
        mut p2t_receiver: Receiver<P2TToPIMessage>,
        p2p_sender: Sender<P2PToPIMessage>,
        mut p2p_receiver: Receiver<P2PToPIMessage>,
    ) -> Result<(), Box<dyn Error>> {
        //
        let tracker_peers_info_table = self.peers_info_table.clone();
        let tracker_listener = TcpListener::bind(SocketAddr::from(SocketAddrV4::new(local_bind_ip, peer_info_sync_open_port))).await?;
        println!("Listening in {:?}", SocketAddr::from(SocketAddrV4::new(local_bind_ip, peer_info_sync_open_port)));
        tokio::spawn(async move {
            let (mut stream, addr) = tracker_listener.accept().await.unwrap();
            let mut buf = vec![0u8; BUF_SIZE];
            loop {
                if let Ok(nbytes) = stream.read(&mut buf).await {
                    // 解析数据包，并修改peers_info_table
                    if nbytes < 5 {
                        continue;
                    }
                    let type_id = buf[0];
                    let payload_bytes = [buf[1], buf[2], buf[3], buf[4]];
                    let payload_length = u32::from_le_bytes(payload_bytes);
                    assert_eq!(payload_length, (nbytes - 5) as u32);
                    match type_id {
                        3u8 => {
                            let peer_info_table = PeersInfoTable::from_bencode(&buf[5..nbytes]).unwrap();
                            println!("Get Peer_info_table: {:?}\n\n\n", peer_info_table);
                            {
                                let mut tracker_peers_info_table_lock = tracker_peers_info_table.lock().await;
                                tracker_peers_info_table_lock.from_set_peer_info_table_packet(peer_info_table);
                            }
                        }
                        _ => {}
                    }
                }
            }
        });
        // TODO 处理通信逻辑
        // lf_receiver
        tokio::spawn(async move {
            loop {
                if let Some(msg) = lf_receiver.recv().await {}
            }
        });
        // p2t_receiver
        tokio::spawn(async move {
            loop {
                if let Some(msg) = p2t_receiver.recv().await {}
            }
        });
        // p2p_receiver
        tokio::spawn(async move {
            loop {
                if let Some(msg) = p2p_receiver.recv().await {}
            }
        });
        Ok(())
    }

    pub fn new(peers_info_table: PeersInfoTable) -> Self {
        Self {
            peers_info_table: Arc::new(Mutex::new(peers_info_table)),
        }
    }
}

pub struct PeersInfoUpdateCommand {}
