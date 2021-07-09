use crate::message::{
    FilePieceInfoAddPayload, PeerDroppedPayload, PeerInfoAddPayload, PeersInfoSetPayload,
    PeersInfoUpdatePayload,
};
use crate::thread_communication_message::{
    KAToPIMessage, LFToPIMessage, P2PToPIMessage, P2TToPIMessage, T2PToPIMessage,
};
use crate::types::{FileDownloader, PeerInfo, PeersInfoTable};
use bendy::decoding::FromBencode;
use bendy::encoding::ToBencode;
use std::collections::HashMap;
use std::error::Error;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;
use tokio::sync::{mpsc, Mutex};

const BUF_SIZE: usize = 1 << 18;

pub struct PeersInfoManagerOfTracker {
    // 全局的Peers Info Table
    peers_info_table: Arc<Mutex<PeersInfoTable>>,
}

impl PeersInfoManagerOfTracker {
    pub async fn start(
        &mut self,
        keep_alive_interval: u64,
        keep_alive_manager_open_port: u16,
        lf_sender: Sender<LFToPIMessage>,
        mut lf_receiver: Receiver<LFToPIMessage>,
        t2p_sender: Sender<T2PToPIMessage>,
        mut t2p_receiver: Receiver<T2PToPIMessage>,
        p2p_sender: Sender<P2PToPIMessage>,
        mut p2p_receiver: Receiver<P2PToPIMessage>,
        ka_sender: Sender<KAToPIMessage>,
        mut ka_receiver: Receiver<KAToPIMessage>,
    ) -> Result<(), Box<dyn Error>> {
        let (download_signal_sender, mut download_signal_receiver) = mpsc::channel::<()>(100);
        // TODO 处理通信逻辑
        // Tracker本机的peer_id默认为 0
        let local_peer_id = 0u32;
        // 记录通信的Peer的Ip地址与sync端口号
        let peers_info_sync_ip_and_port_info_map =
            Arc::new(Mutex::new(HashMap::<u32, SocketAddr>::new()));

        // lf_receiver 与本地文件信息更新与维护模块通信
        let lf_peers_info_table = self.peers_info_table.clone();
        let lf_ip_and_port_info_map = peers_info_sync_ip_and_port_info_map.clone();
        tokio::spawn(async move {
            loop {
                if let Some(message) = lf_receiver.recv().await {
                    log::debug!("[PIT]: Tracker 获取到本地文件更新信息");
                    {
                        let mut lf_peers_info_table_lock = lf_peers_info_table.lock().await;
                        lf_peers_info_table_lock
                            .update_file(message.get_inner_files_meta_info(), local_peer_id);
                        lf_peers_info_table_lock.show_resource_table();
                    }
                    // 发送更新信息
                    let ip_and_port_info_vec;
                    {
                        let ip_and_port_info_map_lock = lf_ip_and_port_info_map.lock().await;
                        ip_and_port_info_vec = ip_and_port_info_map_lock
                            .iter()
                            .map(|(peer_id, socket_addr)| (peer_id.clone(), socket_addr.clone()))
                            .collect::<Vec<(u32, SocketAddr)>>();
                    }
                    // 构造发送的UpdateInfo信息包
                    let mut packet = Vec::<u8>::new();
                    packet.push(4u8);
                    let payload =
                        PeersInfoUpdatePayload::new(0u32, message.get_inner_files_meta_info())
                            .to_bencode()
                            .unwrap();
                    let payload_len = payload.len() as u32;
                    packet.extend_from_slice(&payload_len.to_le_bytes().to_vec());
                    packet.extend_from_slice(&payload);
                    for (id, socket_addr) in ip_and_port_info_vec {
                        log::debug!("[PIT]: Tracker 把本地文件更新信息发送给 peer {}", id);
                        if let Ok(mut stream) = TcpStream::connect(socket_addr).await {
                            let packet_clone = packet.clone();
                            // 广播给每一个Peer
                            tokio::spawn(async move { stream.write(&packet_clone).await });
                        }
                    }
                }
            }
        });

        // t2p_receiver // 与Tracker通信
        let t2p_peers_info_table = self.peers_info_table.clone();
        let t2p_ip_and_port_info_map = peers_info_sync_ip_and_port_info_map.clone();
        tokio::spawn(async move {
            loop {
                if let Some(message) = t2p_receiver.recv().await {
                    match message {
                        T2PToPIMessage::AddOnePeerInfo {
                            peer_info,
                            peer_info_sync_open_port,
                        } => {
                            let peer_local_bind_ip = peer_info.get_peer_ip();
                            let peers_info_table_snapshot;
                            let peer_ip = peer_info.get_peer_ip();
                            let peer_id = peer_info.get_peer_id();
                            {
                                let mut peers_info_table = t2p_peers_info_table.lock().await;
                                peers_info_table.add_one_peer_info(peer_info.clone());
                                peers_info_table_snapshot = peers_info_table.get_a_snapshot();
                            }
                            let ip_and_port_info_map_clone0 = t2p_ip_and_port_info_map.clone();
                            // 获取更新后的表的信息，发送给peer的客户端
                            let t2p_ka_sender = ka_sender.clone();
                            tokio::spawn(async move {
                                let peer_local_bind_ip = [
                                    peer_local_bind_ip[0],
                                    peer_local_bind_ip[1],
                                    peer_local_bind_ip[2],
                                    peer_local_bind_ip[3],
                                ];
                                if let Ok(mut stream) =
                                    TcpStream::connect(SocketAddr::from(SocketAddrV4::new(
                                        Ipv4Addr::from(peer_local_bind_ip),
                                        peer_info_sync_open_port,
                                    )))
                                    .await
                                {
                                    log::debug!(
                                        "[PIT]: Connected with {:?}",
                                        SocketAddr::from(SocketAddrV4::new(
                                            Ipv4Addr::from(peer_local_bind_ip),
                                            peer_info_sync_open_port
                                        ))
                                    );
                                    // 记录Peer的监听地址信息
                                    {
                                        let mut ip_and_port_info_map_lock =
                                            ip_and_port_info_map_clone0.lock().await;
                                        ip_and_port_info_map_lock.insert(
                                            peer_id,
                                            SocketAddr::from(SocketAddrV4::new(
                                                Ipv4Addr::from(peer_local_bind_ip),
                                                peer_info_sync_open_port,
                                            )),
                                        );
                                    }
                                    // 将Peer的IP地址加入KeepAlive模块中维护
                                    let ka_peer_ip = peer_ip.clone();
                                    let ka_peer_ip = [
                                        ka_peer_ip[0],
                                        ka_peer_ip[1],
                                        ka_peer_ip[2],
                                        ka_peer_ip[3],
                                    ];
                                    let ka_message = KAToPIMessage::new_peer_online_message(
                                        peer_id,
                                        Ipv4Addr::from(ka_peer_ip),
                                    );
                                    t2p_ka_sender.send(ka_message).await;

                                    // 发送peer_info文件包
                                    let mut set_peers_info_table_packet = Vec::<u8>::new();
                                    // code 3 => set peers_info_table_packet
                                    set_peers_info_table_packet.push(3u8);
                                    let peers_info_set_payload = PeersInfoSetPayload::new(
                                        peer_id,
                                        peer_ip,
                                        peers_info_table_snapshot,
                                        keep_alive_interval,
                                        keep_alive_manager_open_port,
                                    );
                                    let payload = peers_info_set_payload.to_bencode().unwrap();
                                    let payload_len = payload.len() as u32;
                                    set_peers_info_table_packet
                                        .extend_from_slice(&payload_len.to_le_bytes());
                                    set_peers_info_table_packet.extend_from_slice(&payload);
                                    // 发送包
                                    stream.write(&set_peers_info_table_packet).await.unwrap();
                                    // TODO 告知其他peer新加入的peerInfo
                                    // 广播发送给其他Peer
                                    let ip_and_port_info_vec;
                                    {
                                        let ip_and_port_info_map_lock =
                                            ip_and_port_info_map_clone0.lock().await;
                                        ip_and_port_info_vec = ip_and_port_info_map_lock
                                            .iter()
                                            .map(|(peer_id, socket_addr)| {
                                                (peer_id.clone(), socket_addr.clone())
                                            })
                                            .collect::<Vec<(u32, SocketAddr)>>();
                                    }
                                    // 构造发送的包
                                    let mut packet = Vec::<u8>::new();
                                    packet.push(2u8);
                                    let payload =
                                        PeerInfoAddPayload::new(peer_info).to_bencode().unwrap();
                                    let payload_len = payload.len() as u32;
                                    packet.extend_from_slice(&payload_len.to_le_bytes().to_vec());
                                    packet.extend_from_slice(&payload);
                                    for (id, socket_addr) in ip_and_port_info_vec {
                                        // 原peer可不发广播包
                                        if peer_id == id {
                                            continue;
                                        }
                                        if let Ok(mut stream) =
                                            TcpStream::connect(socket_addr).await
                                        {
                                            let packet_clone = packet.clone();
                                            // 广播给每一个Peer
                                            tokio::spawn(async move {
                                                stream.write(&packet_clone).await
                                            });
                                        }
                                    }
                                }
                            });
                            download_signal_sender.send(()).await;
                        }
                        T2PToPIMessage::UpdatePeerInfo {
                            peer_id,
                            updated_file_meta_info,
                        } => {
                            // 首先更新本地PeersInfoTable
                            {
                                let mut peers_info_table = t2p_peers_info_table.lock().await;
                                peers_info_table
                                    .update_file(updated_file_meta_info.clone(), peer_id);
                            }
                            // 广播发送给其他Peer
                            let ip_and_port_info_vec;
                            {
                                let ip_and_port_info_map_lock =
                                    t2p_ip_and_port_info_map.lock().await;
                                ip_and_port_info_vec = ip_and_port_info_map_lock
                                    .iter()
                                    .map(|(peer_id, socket_addr)| {
                                        (peer_id.clone(), socket_addr.clone())
                                    })
                                    .collect::<Vec<(u32, SocketAddr)>>();
                            }
                            // 构造发送的UpdateInfo信息包
                            let mut packet = Vec::<u8>::new();
                            packet.push(4u8);
                            let payload =
                                PeersInfoUpdatePayload::new(peer_id, updated_file_meta_info)
                                    .to_bencode()
                                    .unwrap();
                            let payload_len = payload.len() as u32;
                            packet.extend_from_slice(&payload_len.to_le_bytes().to_vec());
                            packet.extend_from_slice(&payload);
                            for (id, socket_addr) in ip_and_port_info_vec {
                                // 原peer可不发广播包
                                if peer_id == id {
                                    continue;
                                }
                                if let Ok(mut stream) = TcpStream::connect(socket_addr).await {
                                    let packet_clone = packet.clone();
                                    // 广播给每一个Peer
                                    tokio::spawn(async move { stream.write(&packet_clone).await });
                                }
                            }
                            download_signal_sender.send(()).await;
                        }
                        T2PToPIMessage::UpdatePieceDownloadedInfo {
                            peer_id,
                            file_piece_info,
                            file_meta_info_with_empty_piece,
                        } => {
                            // 首先更新本地PeersInfoTable
                            log::debug!("[PIT]: 收到peer {} 的文件块更新信息", peer_id);
                            {
                                let mut peers_info_table = t2p_peers_info_table.lock().await;
                                peers_info_table.add_file_piece_info(
                                    peer_id,
                                    file_meta_info_with_empty_piece.clone(),
                                    file_piece_info.clone(),
                                );
                            }
                            // 广播发送给其他Peer
                            let ip_and_port_info_vec;
                            {
                                let ip_and_port_info_map_lock =
                                    t2p_ip_and_port_info_map.lock().await;
                                ip_and_port_info_vec = ip_and_port_info_map_lock
                                    .iter()
                                    .map(|(peer_id, socket_addr)| {
                                        (peer_id.clone(), socket_addr.clone())
                                    })
                                    .collect::<Vec<(u32, SocketAddr)>>();
                            }
                            // 构造发送的UpdateInfo信息包
                            let mut packet = Vec::<u8>::new();
                            packet.push(8u8);
                            let payload = FilePieceInfoAddPayload::new(
                                peer_id,
                                file_piece_info,
                                file_meta_info_with_empty_piece,
                            )
                            .to_bencode()
                            .unwrap();
                            let payload_len = payload.len() as u32;
                            packet.extend_from_slice(&payload_len.to_le_bytes().to_vec());
                            packet.extend_from_slice(&payload);
                            log::debug!("广播peer {} 的新添加块信息", peer_id);
                            for (id, socket_addr) in ip_and_port_info_vec {
                                // 原peer可不发广播包
                                if peer_id == id {
                                    continue;
                                }
                                if let Ok(mut stream) = TcpStream::connect(socket_addr).await {
                                    let packet_clone = packet.clone();
                                    // 广播给每一个Peer
                                    tokio::spawn(async move { stream.write(&packet_clone).await });
                                }
                            }
                        }
                    }
                }
            }
        });

        // p2p_receiver 处理与P2P模块的通信
        let p2p_peers_info_sync_ip_and_port_info_map = peers_info_sync_ip_and_port_info_map.clone();
        let p2p_peers_info_table = self.peers_info_table.clone();
        tokio::spawn(async move {
            loop {
                if let Some(message) = p2p_receiver.recv().await {
                    match message {
                        P2PToPIMessage::PieceDownloadedInfo {
                            file_meta_info_with_empty_piece_info,
                            file_piece_info,
                        } => {
                            // 接收到文件片更新的消息，首先更新本地的PeerInfoTable，对于tracker而言则给所有Peer发送广播，对于peer而言，则发送给tracker并让tracker广播
                            {
                                let mut p2p_peers_info_table_lock =
                                    p2p_peers_info_table.lock().await;
                                // Tracker的peer_id为0
                                p2p_peers_info_table_lock.add_file_piece_info(
                                    0,
                                    file_meta_info_with_empty_piece_info.clone(),
                                    file_piece_info.clone(),
                                );
                            }
                            // 广播出去
                            let p2p_ip_and_port_info_vec;
                            {
                                let p2p_peers_info_sync_ip_and_port_info_map_lock =
                                    p2p_peers_info_sync_ip_and_port_info_map.lock().await;
                                p2p_ip_and_port_info_vec =
                                    p2p_peers_info_sync_ip_and_port_info_map_lock
                                        .iter()
                                        .map(|(peer_id, socket_addr)| {
                                            (peer_id.clone(), socket_addr.clone())
                                        })
                                        .collect::<Vec<(u32, SocketAddr)>>();
                            }
                            // 发送用户信息包
                            let mut packet = Vec::<u8>::new();
                            packet.push(8u8);
                            let payload = FilePieceInfoAddPayload::new(
                                0,
                                file_piece_info,
                                file_meta_info_with_empty_piece_info,
                            )
                            .to_bencode()
                            .unwrap();
                            let payload_len = payload.len() as u32;
                            packet.extend_from_slice(&payload_len.to_le_bytes().to_vec());
                            packet.extend_from_slice(&payload);
                            // 给每个peer发送广播包
                            for (_, socket_addr) in p2p_ip_and_port_info_vec {
                                if let Ok(mut stream) = TcpStream::connect(socket_addr).await {
                                    let packet_clone = packet.clone();
                                    // 广播给每一个Peer
                                    tokio::spawn(async move { stream.write(&packet_clone).await });
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        });

        // 处理下载文件列表，并交由p2p模块下载
        let p2p_distributing_peers_info_table = self.peers_info_table.clone();
        tokio::spawn(async move {
            // 定时扫描，获取peer_id未拥有的文件的sha3_code
            loop {
                // 3秒扫描一次吧
                if let Some(_) = download_signal_receiver.recv().await {
                    log::debug!("[PIT]: 收到下载文件信号");
                    let peers_info_table_snapshot;
                    {
                        let p2p_distributing_peers_info_table =
                            p2p_distributing_peers_info_table.lock().await;
                        peers_info_table_snapshot =
                            p2p_distributing_peers_info_table.get_a_snapshot();
                    }
                    let download_file_sha3_code_list =
                        peers_info_table_snapshot.get_download_file_sha3_code_list(0);
                    for download_file_sha3_code in download_file_sha3_code_list {
                        // 每次下载都更新peers_info_table
                        let download_file_peers_info_table_snapshot;
                        {
                            let p2p_distributing_peers_info_table =
                                p2p_distributing_peers_info_table.lock().await;
                            download_file_peers_info_table_snapshot =
                                p2p_distributing_peers_info_table.get_a_snapshot();
                        }
                        download_file_peers_info_table_snapshot.show_resource_table();
                        let (sender, receiver) = oneshot::channel::<()>();
                        let message = P2PToPIMessage::new_downloaded_file_info_message(
                            download_file_sha3_code,
                            download_file_peers_info_table_snapshot,
                            sender,
                        );
                        p2p_sender.send(message).await;
                        // 等待文件下载成功
                        if let Ok(_) = receiver.await {
                            log::info!("[PIT]: 成功下载");
                        }
                    }
                }
            }
        });

        // ka_receiver 处理与KeepAlive模块的通信
        let ka_peers_info_table = self.peers_info_table.clone();
        let ka_peers_info_sync_ip_and_port_info_map = peers_info_sync_ip_and_port_info_map.clone();
        tokio::spawn(async move {
            loop {
                if let Some(message) = ka_receiver.recv().await {
                    match message {
                        KAToPIMessage::PeerDroppedMessage { peer_id } => {
                            // peers_info_table处理掉线信息
                            {
                                let mut ka_peers_info_table_lock = ka_peers_info_table.lock().await;
                                ka_peers_info_table_lock.handle_peer_dropped(peer_id);
                                ka_peers_info_table_lock.show_resource_table();
                            }
                            log::debug!("[KAT]: 发现掉线Peer，本地信息已修改");
                            // 给其他用户发送
                            let ka_ip_and_port_info_vec;
                            {
                                let mut peers_info_sync_ip_and_port_info_map_lock =
                                    ka_peers_info_sync_ip_and_port_info_map.lock().await;
                                peers_info_sync_ip_and_port_info_map_lock.remove(&peer_id);
                                ka_ip_and_port_info_vec = peers_info_sync_ip_and_port_info_map_lock
                                    .iter()
                                    .map(|(peer_id, socket_addr)| {
                                        (peer_id.clone(), socket_addr.clone())
                                    })
                                    .collect::<Vec<(u32, SocketAddr)>>();
                            }
                            // 发送用户掉线信息包
                            let mut packet = Vec::<u8>::new();
                            packet.push(5u8);
                            let payload = PeerDroppedPayload::new(peer_id).to_bencode().unwrap();
                            let payload_len = payload.len() as u32;
                            packet.extend_from_slice(&payload_len.to_le_bytes().to_vec());
                            packet.extend_from_slice(&payload);
                            for (_, socket_addr) in ka_ip_and_port_info_vec {
                                // 原peer可不发广播包
                                if let Ok(mut stream) = TcpStream::connect(socket_addr).await {
                                    let packet_clone = packet.clone();
                                    // 广播给每一个Peer
                                    tokio::spawn(async move { stream.write(&packet_clone).await });
                                }
                            }
                        }
                        _ => {}
                    }
                }
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
    peers_info_table: Arc<Mutex<PeersInfoTable>>,
}

impl PeersInfoManagerOfPeer {
    pub async fn start(
        &mut self,
        local_bind_ip: Ipv4Addr,
        peer_info_sync_open_port: u16,
        local_keep_alive_open_port: u16,
        tracker_ip: Ipv4Addr,
        lf_sender: Sender<LFToPIMessage>,
        mut lf_receiver: Receiver<LFToPIMessage>,
        p2t_sender: Sender<P2TToPIMessage>,
        mut p2t_receiver: Receiver<P2TToPIMessage>,
        p2p_sender: Sender<P2PToPIMessage>,
        mut p2p_receiver: Receiver<P2PToPIMessage>,
    ) -> Result<(), Box<dyn Error>> {
        let (download_signal_sender, mut download_signal_receiver) = mpsc::channel::<()>(100);
        //
        let mut keep_alive_interval = Arc::new(Mutex::new(0u64));

        let tracker_peers_info_table = self.peers_info_table.clone();
        let tracker_listener = TcpListener::bind(SocketAddr::from(SocketAddrV4::new(
            local_bind_ip,
            peer_info_sync_open_port,
        )))
        .await?;
        let mut local_peer_id = Arc::new(Mutex::<Option<u32>>::new(None));
        let mut local_peer_ip = Arc::new(Mutex::<Option<Vec<u8>>>::new(None));
        let mut is_keep_alive_manager_start = Arc::new(Mutex::new(false));
        let (keep_alive_interval_sender, mut keep_alive_interval_receiver) =
            mpsc::channel::<(u64, u16)>(5);
        // 开启keep_alive_manager
        let is_keep_alive_manager_start_clone2 = is_keep_alive_manager_start.clone();
        tokio::spawn(async move {
            if let Some((keep_alive_interval, keep_alive_manager_open_port)) =
                keep_alive_interval_receiver.recv().await
            {
                let udp_socket = UdpSocket::bind(SocketAddr::from(SocketAddrV4::new(
                    local_bind_ip,
                    local_keep_alive_open_port,
                )))
                .await
                .unwrap();
                let heart_beat_packet = vec![0, 0, 0, 0, 0];
                {
                    let mut is_keep_alive_manager_start_lock =
                        is_keep_alive_manager_start_clone2.lock().await;
                    *is_keep_alive_manager_start_lock = true;
                }
                loop {
                    udp_socket
                        .send_to(
                            &heart_beat_packet,
                            SocketAddr::from(SocketAddrV4::new(
                                tracker_ip,
                                keep_alive_manager_open_port,
                            )),
                        )
                        .await;
                    tokio::time::sleep(Duration::from_millis(keep_alive_interval / 2)).await;
                }
            }
        });
        log::info!(
            "[PIP]: Peer Listening on {:?}",
            SocketAddr::from(SocketAddrV4::new(local_bind_ip, peer_info_sync_open_port))
        );
        let local_peer_id_clone = local_peer_id.clone();
        let local_peer_ip_clone = local_peer_ip.clone();
        let is_keep_alive_manager_start_clone = is_keep_alive_manager_start.clone();
        tokio::spawn(async move {
            loop {
                let (mut stream, addr) = tracker_listener.accept().await.unwrap();
                let mut buf = vec![0u8; BUF_SIZE];

                let mut is_packet_header_coming = false;
                let mut packet_expected_length = u32::MAX - 10000;
                let mut type_id = 29;
                let mut packet = Vec::new();
                while let Ok(nbytes) = stream.read(&mut buf[..]).await {
                    if is_packet_header_coming {
                        packet.extend_from_slice(&buf[..nbytes]);
                        if packet.len() == packet_expected_length as usize {
                            break;
                        }
                    } else {
                        if nbytes < 5 {
                            continue;
                        }
                        type_id = buf[0];
                        let payload_bytes = [buf[1], buf[2], buf[3], buf[4]];
                        packet.extend_from_slice(&buf[..nbytes]);
                        let payload_length = u32::from_le_bytes(payload_bytes);
                        packet_expected_length = payload_length + 5;
                        is_packet_header_coming = true;
                        if packet_expected_length as usize == packet.len() {
                            break;
                        }
                    }
                }
                match type_id {
                    2u8 => {
                        if let Ok(peer_info_add_payload) =
                            PeerInfoAddPayload::from_bencode(&packet[5..])
                        {
                            let peer_info = peer_info_add_payload.get_inner_peer_info();
                            log::info!("获得新加入的用户信息, peer {}", peer_info.get_peer_id());
                            {
                                let mut tracker_peers_info_table_lock =
                                    tracker_peers_info_table.lock().await;
                                tracker_peers_info_table_lock.add_one_peer_info(peer_info);
                                tracker_peers_info_table_lock.show_resource_table();
                            }
                            download_signal_sender.send(()).await;
                        }
                    }
                    3u8 => {
                        let peers_info_set_payload =
                            PeersInfoSetPayload::from_bencode(&packet[5..]).unwrap();
                        // 获得peer_id与peer_ip
                        {
                            let mut peer_id_lock = local_peer_id_clone.lock().await;
                            if peer_id_lock.is_none() {
                                let mut peer_id = peer_id_lock;
                                peer_id.replace(peers_info_set_payload.get_peer_id());
                            }
                        }
                        {
                            let mut peer_ip_lock = local_peer_ip_clone.lock().await;
                            if peer_ip_lock.is_none() {
                                let mut peer_ip = peer_ip_lock;
                                peer_ip.replace(peers_info_set_payload.get_peer_ip());
                            }
                        }
                        {
                            let mut tracker_peers_info_table_lock =
                                tracker_peers_info_table.lock().await;
                            tracker_peers_info_table_lock.update_from_set_peer_info_table_packet(
                                peers_info_set_payload.get_peers_info_table(),
                            );
                            tracker_peers_info_table_lock.show_resource_table();
                        }
                        let keep_alive_manager_open_port =
                            peers_info_set_payload.get_keep_alive_manager_open_port();
                        let keep_alive_interval = peers_info_set_payload.get_keep_alive_interval();
                        // 尝试开启keep_alive_manager
                        {
                            let mut is_keep_alive_manager_start_lock =
                                is_keep_alive_manager_start_clone.lock().await;
                            if !(*is_keep_alive_manager_start_lock) {
                                log::info!("[PIP]: 开启PeerKeepAliveManager");
                                keep_alive_interval_sender
                                    .send((keep_alive_interval, keep_alive_manager_open_port))
                                    .await;
                            }
                        }
                        download_signal_sender.send(()).await;
                    }
                    4u8 => {
                        if let Ok(peers_info_update_payload) =
                            PeersInfoUpdatePayload::from_bencode(&packet[5..])
                        {
                            let peer_id = peers_info_update_payload.get_peer_id();
                            log::info!(
                                "[PIP]: 获得一个Peer的文件添加信息 from Tracker, Peer id {}",
                                peer_id
                            );
                            let updated_file_meta_info =
                                peers_info_update_payload.get_updated_file_meta_info();
                            {
                                let mut tracker_peers_info_table_lock =
                                    tracker_peers_info_table.lock().await;
                                tracker_peers_info_table_lock
                                    .update_file(updated_file_meta_info, peer_id);
                                tracker_peers_info_table_lock.show_resource_table();
                            }
                        }
                        download_signal_sender.send(()).await;
                    }
                    5u8 => {
                        // 获得一个peer的掉线信息
                        if let Ok(peer_dropped_payload) =
                            PeerDroppedPayload::from_bencode(&packet[5..])
                        {
                            let dropped_peer_id = peer_dropped_payload.get_dropped_peer_id();
                            {
                                let mut tracker_peers_info_table_lock =
                                    tracker_peers_info_table.lock().await;
                                tracker_peers_info_table_lock.handle_peer_dropped(dropped_peer_id);
                                tracker_peers_info_table_lock.show_resource_table();
                            }
                        }
                    }
                    8u8 => {
                        // 获得一个peer的新增块信息
                        if let Ok(piece_info_add_payload) =
                            FilePieceInfoAddPayload::from_bencode(&packet[5..])
                        {
                            let (peer_id, file_piece_info, file_meta_info_with_empty_piece) =
                                piece_info_add_payload.get_inner_info();
                            {
                                let mut tracker_peers_info_table_lock =
                                    tracker_peers_info_table.lock().await;
                                tracker_peers_info_table_lock.add_file_piece_info(
                                    peer_id,
                                    file_meta_info_with_empty_piece,
                                    file_piece_info,
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
        });
        // TODO 处理通信逻辑
        // lf_receiver
        let lf_peers_info_table = self.peers_info_table.clone();
        let lf_local_peer_id_clone = local_peer_id.clone();
        let lf_p2t_sender = p2t_sender.clone();
        tokio::spawn(async move {
            loop {
                if let Some(message) = lf_receiver.recv().await {
                    log::info!("[PIP]: Peer 获得本地文件更新信息");
                    {
                        // TODO 这里可能会出现死锁
                        let mut local_peer_id_lock = lf_local_peer_id_clone.lock().await;
                        if local_peer_id_lock.is_none() {
                            // 未检测到peer_id
                            log::warn!("[PIP]: 未检测到peer_id");
                            continue;
                        } else {
                            let peer_id = local_peer_id_lock.as_ref().unwrap().clone();
                            drop(local_peer_id_lock);
                            {
                                let mut lf_peers_info_table_lock = lf_peers_info_table.lock().await;
                                lf_peers_info_table_lock
                                    .update_file(message.get_inner_files_meta_info(), peer_id);
                                lf_peers_info_table_lock.show_resource_table();
                            }
                            // 给p2t发个更新消息
                            // TODO this
                            log::info!("[PIP]: peer将本地更新信息发送给tracker");
                            lf_p2t_sender
                                .send(P2TToPIMessage::new_update_one_peer_info(
                                    peer_id,
                                    message.get_inner_files_meta_info(),
                                ))
                                .await;
                        }
                    }
                }
            }
        });
        // p2t_receiver
        tokio::spawn(async move {
            loop {
                if let Some(msg) = p2t_receiver.recv().await {}
            }
        });
        // p2p_receiver
        let p2p_peers_info_table = self.peers_info_table.clone();
        let p2p_local_peer_id_clone = local_peer_id.clone();
        let p2p_p2t_sender = p2t_sender.clone();
        tokio::spawn(async move {
            loop {
                if let Some(message) = p2p_receiver.recv().await {
                    match message {
                        P2PToPIMessage::PieceDownloadedInfo {
                            file_meta_info_with_empty_piece_info,
                            file_piece_info,
                        } => {
                            // 接收到文件片更新的消息，首先更新本地的PeerInfoTable，对于tracker而言则给所有Peer发送广播，对于peer而言，则发送给tracker并让tracker广播
                            // 获取local_peer_id
                            let local_peer_id;
                            {
                                let local_peer_id_lock = p2p_local_peer_id_clone.lock().await;
                                local_peer_id = local_peer_id_lock.as_ref().unwrap().clone();
                                // 此时一定已经获得了peer_id
                            }
                            {
                                let mut p2p_peers_info_table_lock =
                                    p2p_peers_info_table.lock().await;
                                // Tracker的peer_id为0
                                p2p_peers_info_table_lock.add_file_piece_info(
                                    local_peer_id,
                                    file_meta_info_with_empty_piece_info.clone(),
                                    file_piece_info.clone(),
                                );
                            }
                            // 发给tracker本地块更新的信息，交由p2tmanager来完成
                            let message = P2TToPIMessage::new_update_piece_downloaded_info(
                                local_peer_id,
                                file_piece_info,
                                file_meta_info_with_empty_piece_info,
                            );
                            p2p_p2t_sender.send(message).await;
                        }
                        _ => {}
                    }
                }
            }
        });

        // 处理下载文件列表，并交由p2p模块下载
        let p2p_distributing_peers_info_table = self.peers_info_table.clone();
        let p2p_local_peer_id = local_peer_id.clone();
        tokio::spawn(async move {
            // 定时扫描，获取peer_id未拥有的文件的sha3_code
            loop {
                if let Some(_) = download_signal_receiver.recv().await {
                    log::info!("[PIP]: 收到下载文件信号");
                    let peers_info_table_snapshot;
                    {
                        let p2p_distributing_peers_info_table =
                            p2p_distributing_peers_info_table.lock().await;
                        peers_info_table_snapshot =
                            p2p_distributing_peers_info_table.get_a_snapshot();
                    }
                    let local_peer_id;
                    {
                        let p2p_local_peer_id_lock = p2p_local_peer_id.lock().await;
                        local_peer_id = *p2p_local_peer_id_lock;
                    }
                    let download_file_sha3_code_list = peers_info_table_snapshot
                        .get_download_file_sha3_code_list(local_peer_id.unwrap());
                    log::info!("下载文件数量： {}", download_file_sha3_code_list.len());
                    for download_file_sha3_code in download_file_sha3_code_list {
                        // 每次下载都更新peers_info_table
                        let download_file_peers_info_table_snapshot;
                        {
                            let p2p_distributing_peers_info_table =
                                p2p_distributing_peers_info_table.lock().await;
                            download_file_peers_info_table_snapshot =
                                p2p_distributing_peers_info_table.get_a_snapshot();
                        }
                        download_file_peers_info_table_snapshot.show_resource_table();
                        let (sender, receiver) = oneshot::channel::<()>();
                        let message = P2PToPIMessage::new_downloaded_file_info_message(
                            download_file_sha3_code,
                            download_file_peers_info_table_snapshot,
                            sender,
                        );
                        p2p_sender.send(message).await;
                        // 等待文件下载成功
                        if let Ok(_) = receiver.await {
                            log::info!("成功下载");
                        }
                    }
                }
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
