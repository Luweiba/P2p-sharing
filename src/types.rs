use crate::message::FilePieceRequestPayload;
use crate::peer_to_peer::FilePieceDownloadedBuffer;
use crate::thread_communication_message::{P2PToLFMessage, P2PToPIMessage};
use bendy::decoding::{FromBencode, Object, ResultExt};
use bendy::encoding::{SingleItemEncoder, ToBencode};
use sha3::{Digest, Sha3_256};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::ffi::OsString;
use std::fmt::{Display, Formatter};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;
use tokio::sync::{mpsc, Mutex};

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct FileMetaInfo {
    sha3_code: Vec<u8>,
    file_name: OsString,
    file_piece_size: u32,
    pieces_info: Vec<FilePieceInfo>,
}

impl FileMetaInfo {
    pub fn new(
        sha3_code: Vec<u8>,
        file_name: OsString,
        file_piece_size: u32,
        pieces_info: Vec<FilePieceInfo>,
    ) -> Self {
        Self {
            sha3_code,
            file_name,
            file_piece_size,
            pieces_info,
        }
    }

    pub fn get_file_sha3_code(&self) -> Vec<u8> {
        self.sha3_code.clone()
    }

    pub fn get_file_name(&self) -> OsString {
        self.file_name.clone()
    }
}
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct FilePieceInfo {
    index: u32,
    sha3_code: Vec<u8>,
}

impl FilePieceInfo {
    pub fn new(index: u32, sha3_code: Vec<u8>) -> Self {
        Self { index, sha3_code }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct PeerInfo {
    peer_id: u32,
    peer_ip: Vec<u8>,
    peer_open_port: u16,
    peer_file_meta_info_report: Vec<FileMetaInfo>,
}

impl PeerInfo {
    pub fn new(
        peer_id: u32,
        peer_ip: Vec<u8>,
        peer_open_port: u16,
        peer_file_meta_info_report: Vec<FileMetaInfo>,
    ) -> Self {
        assert_eq!(peer_ip.len(), 4);
        Self {
            peer_id,
            peer_ip,
            peer_open_port,
            peer_file_meta_info_report,
        }
    }

    pub fn get_peer_ip(&self) -> Vec<u8> {
        self.peer_ip.clone()
    }

    pub fn get_peer_id(&self) -> u32 {
        self.peer_id
    }
}
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct PeersInfoTable {
    // 全局PeersInfo Table
    peers_info: Vec<PeerInfo>,
    // sha3_code => FileMetaInfo
    file_meta_info_hash_map: HashMap<Vec<u8>, FileMetaInfo>,
    // sha3_code => Peer_id 进行映射，用于全局去重，
    files_info_hash_map: HashMap<Vec<u8>, Vec<u32>>, // 全局表, 与本地的表进行同步， //TODO 或许local_file_manager不需要此字段（！！还是需要同步的）
    // file_piece_sha3_code => Peer_id
    files_piece_info_hash_map: HashMap<Vec<u8>, Vec<u32>>,
    // peer_id => peer_socket_addr
    peer_socket_addr_hash_map: HashMap<u32, SocketAddr>,
}

impl PeersInfoTable {
    pub fn new() -> Self {
        Self {
            peers_info: Vec::new(),
            file_meta_info_hash_map: HashMap::new(),
            files_info_hash_map: HashMap::new(),
            files_piece_info_hash_map: HashMap::new(),
            peer_socket_addr_hash_map: HashMap::new(),
        }
    }

    pub fn from_peers_info(peers_info: Vec<PeerInfo>) -> Self {
        let mut file_meta_info_hash_map = HashMap::new();
        let mut files_info_hash_map = HashMap::new();
        let mut files_piece_info_hash_map = HashMap::new();
        let mut peer_socket_addr_hash_map = HashMap::new();
        for peer_info in peers_info.iter() {
            let peer_id = peer_info.peer_id;
            let peer_ip = peer_info.peer_ip.clone();
            let peer_open_port = peer_info.peer_open_port;
            // 添加 peer_id => socket_addr
            peer_socket_addr_hash_map.insert(
                peer_id,
                SocketAddr::from(SocketAddrV4::new(
                    Ipv4Addr::new(peer_ip[0], peer_ip[1], peer_ip[2], peer_ip[3]),
                    peer_open_port,
                )),
            );
            for file_meta_info in peer_info.peer_file_meta_info_report.iter() {
                let file_sha3_code = file_meta_info.sha3_code.clone();
                if !file_meta_info_hash_map.contains_key(&file_sha3_code) {
                    let file_meta_info_clone = file_meta_info.clone();
                    file_meta_info_hash_map.insert(file_sha3_code.clone(), file_meta_info_clone);
                }
                let file_name = file_meta_info.file_name.clone();
                let file_peer_id_vec = files_info_hash_map.entry(file_sha3_code).or_insert(vec![]);
                file_peer_id_vec.push(peer_id);
                for file_piece_meta_info in file_meta_info.pieces_info.iter() {
                    let file_piece_sha3_code = file_piece_meta_info.sha3_code.clone();
                    let file_piece_peer_id_vec = files_piece_info_hash_map
                        .entry(file_piece_sha3_code)
                        .or_insert(vec![]);
                    file_piece_peer_id_vec.push(peer_id);
                }
            }
        }

        Self {
            peers_info,
            file_meta_info_hash_map,
            files_info_hash_map,
            files_piece_info_hash_map,
            peer_socket_addr_hash_map,
        }
    }
    // 用于Tracker的初始化
    pub fn tracker_new(
        tracker_ip: Vec<u8>,
        peer_open_port: u16,
        files_meta_info: Vec<FileMetaInfo>,
    ) -> Self {
        let peer_info = PeerInfo::new(0, tracker_ip, peer_open_port, files_meta_info);
        Self::from_peers_info(vec![peer_info])
    }

    // set_peer_info_table
    pub fn update_from_set_peer_info_table_packet(&mut self, peer_info_table: PeersInfoTable) {
        self.peers_info = peer_info_table.peers_info;
        self.files_piece_info_hash_map = peer_info_table.files_piece_info_hash_map;
        self.files_info_hash_map = peer_info_table.files_info_hash_map;
        self.peer_socket_addr_hash_map = peer_info_table.peer_socket_addr_hash_map;
        self.file_meta_info_hash_map = peer_info_table.file_meta_info_hash_map;
    }
    // 修改peers_info信息
    pub fn update_file(&mut self, updated_file_meta_info: Vec<FileMetaInfo>, peer_id: u32) {
        println!("update file info");
        // 更新peer_info
        for peer_info in self.peers_info.iter_mut() {
            if peer_info.peer_id == peer_id {
                peer_info
                    .peer_file_meta_info_report
                    .extend_from_slice(&updated_file_meta_info);
            }
        }
        // 更新hashmap
        for file_meta_info in updated_file_meta_info {
            let sha3_code = file_meta_info.sha3_code.clone();
            if !self.file_meta_info_hash_map.contains_key(&sha3_code) {
                self.file_meta_info_hash_map
                    .insert(sha3_code.clone(), file_meta_info.clone());
            }
            let file_peer_id_vec = self.files_info_hash_map.entry(sha3_code).or_insert(vec![]);
            file_peer_id_vec.push(peer_id);
            for file_piece_meta_info in file_meta_info.pieces_info {
                let piece_sha3_code = file_piece_meta_info.sha3_code;
                let file_piece_peer_id_vec = self
                    .files_piece_info_hash_map
                    .entry(piece_sha3_code)
                    .or_insert(vec![]);
                file_piece_peer_id_vec.push(peer_id);
            }
        }
    }
    // 处理file_piece_info
    pub fn add_file_piece_info(
        &mut self,
        peer_id: u32,
        mut file_meta_info_with_empty_piece: FileMetaInfo,
        file_piece_info: FilePieceInfo,
    ) {
        // 在peer_info表中添加块信息
        println!("添加一个文件块信息");
        let file_sha3_code = file_meta_info_with_empty_piece.get_file_sha3_code();
        for peer_info in self.peers_info.iter_mut() {
            if peer_info.peer_id == peer_id {
                let mut is_first_piece_of_file = true;
                for file_meta_info in peer_info.peer_file_meta_info_report.iter_mut() {
                    if file_meta_info.sha3_code == file_sha3_code {
                        println!("添加文件的其他块");
                        file_meta_info.pieces_info.push(file_piece_info.clone());
                        is_first_piece_of_file = false;
                        break;
                    }
                }
                if is_first_piece_of_file {
                    println!("添加文件的第一个块");
                    file_meta_info_with_empty_piece
                        .pieces_info
                        .push(file_piece_info.clone());
                    peer_info
                        .peer_file_meta_info_report
                        .push(file_meta_info_with_empty_piece.clone());
                    let mut file_peer_id_vec = self
                        .files_info_hash_map
                        .entry(file_sha3_code.clone())
                        .or_insert(vec![]);
                    file_peer_id_vec.push(peer_id);
                }
            }
        }
        // self.file_meta_info_hash_map 肯定有相关块的信息
        // self.peer_socket_addr_hash_map 地址信息不必更新
        // 肯定有peer_id_vec
        let file_piece_sha3_code = file_piece_info.sha3_code.clone();
        let mut file_piece_peer_id_vec = self
            .files_piece_info_hash_map
            .entry(file_piece_sha3_code)
            .or_insert(vec![]);
        file_piece_peer_id_vec.push(peer_id);
    }
    // 处理接收的peerInfo信息
    pub fn add_one_peer_info(&mut self, peer_info: PeerInfo) {
        let peer_id = peer_info.peer_id;
        let peer_ip = peer_info.peer_ip.clone();
        let peer_open_port = peer_info.peer_open_port;
        let peer_file_meta_info_report = peer_info.peer_file_meta_info_report.clone();
        // 添加 peer_id => socket_addr
        self.peer_socket_addr_hash_map.insert(
            peer_id,
            SocketAddr::from(SocketAddrV4::new(
                Ipv4Addr::new(peer_ip[0], peer_ip[1], peer_ip[2], peer_ip[3]),
                peer_open_port,
            )),
        );
        for meta_info in peer_file_meta_info_report {
            // 处理文件去重信息
            let sha3_code = meta_info.sha3_code.clone();
            if !self.file_meta_info_hash_map.contains_key(&sha3_code) {
                self.file_meta_info_hash_map
                    .insert(sha3_code.clone(), meta_info.clone());
            }
            let peer_id_vec = self.files_info_hash_map.entry(sha3_code).or_insert(vec![]);
            peer_id_vec.push(peer_id);
            // 处理文件的片去重的信息
            for piece_info in meta_info.pieces_info {
                let piece_sha3_code = piece_info.sha3_code;
                let piece_peer_id_vec = self
                    .files_piece_info_hash_map
                    .entry(piece_sha3_code)
                    .or_insert(vec![]);
                piece_peer_id_vec.push(peer_id);
            }
        }

        self.peers_info.push(peer_info);
    }
    // 获取一份拷贝
    pub fn get_a_snapshot(&self) -> Self {
        self.clone()
    }

    pub fn handle_peer_dropped(&mut self, dropped_peer_id: u32) {
        let mut dropped_peer_index = self.peers_info.len();
        // 删除peer_id_socket信息
        self.peer_socket_addr_hash_map.remove(&dropped_peer_id);
        for (index, peer_info) in self.peers_info.iter().enumerate() {
            if peer_info.peer_id == dropped_peer_id {
                dropped_peer_index = index;
            }
            // 删除存储在Hash表中的Peer信息
            let file_meta_info_report = &peer_info.peer_file_meta_info_report;
            for file_meta_info in file_meta_info_report {
                let file_sha3_code = &file_meta_info.sha3_code;
                let mut delete_pair_flag = false;
                if let Some(file_peer_id_vec) = self.files_info_hash_map.get_mut(file_sha3_code) {
                    let mut dropped_index = file_peer_id_vec.len();
                    for (index, peer_id) in file_peer_id_vec.iter().enumerate() {
                        if *peer_id == dropped_peer_id {
                            dropped_index = index;
                            break;
                        }
                    }
                    if dropped_index < file_peer_id_vec.len() {
                        file_peer_id_vec.remove(dropped_index);
                        if file_peer_id_vec.is_empty() {
                            delete_pair_flag = true;
                        }
                    }
                }
                if delete_pair_flag {
                    self.files_info_hash_map.remove(file_sha3_code);
                    self.file_meta_info_hash_map.remove(file_sha3_code);
                }
                // 处理file_piece的Hash表中的Peer信息
                let file_piece_meta_info_report = &file_meta_info.pieces_info;
                for file_piece_meta_info in file_piece_meta_info_report {
                    let file_piece_sha3_code = &file_piece_meta_info.sha3_code;
                    let mut delete_pair_flag = false;
                    if let Some(file_piece_peer_id_vec) =
                        self.files_piece_info_hash_map.get_mut(file_piece_sha3_code)
                    {
                        let mut dropped_index = file_piece_peer_id_vec.len();
                        for (index, peer_id) in file_piece_peer_id_vec.iter().enumerate() {
                            if *peer_id == dropped_peer_id {
                                dropped_index = index;
                                break;
                            }
                        }
                        if dropped_index < file_piece_peer_id_vec.len() {
                            file_piece_peer_id_vec.remove(dropped_index);
                            if file_piece_peer_id_vec.is_empty() {
                                delete_pair_flag = true;
                            }
                        }
                    }
                    if delete_pair_flag {
                        self.files_piece_info_hash_map.remove(file_piece_sha3_code);
                    }
                }
            }
        }
        if dropped_peer_index < self.peers_info.len() {
            self.peers_info.remove(dropped_peer_index);
        }
    }

    pub fn show_resource_table(&self) {
        println!("################# 打印ResourceTable ##################");
        let mut sha3_code_set = HashSet::new();
        let mut resource_table = ResourceTable::new();
        for peer_info in self.peers_info.iter() {
            let file_meta_info_report = &peer_info.peer_file_meta_info_report;
            for file_meta_info in file_meta_info_report {
                let sha3_code = &file_meta_info.sha3_code;
                let file_name = &file_meta_info.file_name;
                if sha3_code_set.insert(sha3_code.clone()) {
                    let peer_ids = self.files_info_hash_map.get(sha3_code).unwrap();
                    resource_table.add_one_resource(Resource::new(
                        file_name.clone(),
                        sha3_code.clone(),
                        peer_ids.clone(),
                    ));
                }
            }
        }
        resource_table.display();
    }

    pub fn get_download_file_sha3_code_list(&self, local_peer_id: u32) -> Vec<Vec<u8>> {
        let mut download_file_sha3_code_list = Vec::new();
        for (file_sha3_code, peer_id_vec) in self.files_info_hash_map.iter() {
            let mut is_needed = true;
            for peer_id in peer_id_vec {
                if *peer_id == local_peer_id {
                    is_needed = false;
                }
            }
            if is_needed {
                download_file_sha3_code_list.push(file_sha3_code.clone());
            }
        }
        download_file_sha3_code_list
    }
}

struct ResourceTable {
    resource_table: Vec<Resource>,
}

impl ResourceTable {
    pub fn new() -> Self {
        Self {
            resource_table: Vec::new(),
        }
    }

    pub fn add_one_resource(&mut self, resource: Resource) {
        self.resource_table.push(resource);
    }

    pub fn display(&self) {
        for resource in self.resource_table.iter() {
            println!(
                "{:?}\t\t{:?}",
                resource.resource_name, resource.resource_owner
            );
        }
    }
}

struct Resource {
    resource_name: OsString,
    resource_sha3_code: Vec<u8>,
    resource_owner: Vec<u32>,
}

impl Resource {
    pub fn new(
        resource_name: OsString,
        resource_sha3_code: Vec<u8>,
        resource_owner: Vec<u32>,
    ) -> Self {
        Self {
            resource_name,
            resource_sha3_code,
            resource_owner,
        }
    }
}

pub struct FileDownloader {
    file_meta_info: FileMetaInfo,
    file_piece_owner_table: FilePieceOwnerTable,
}

impl FileDownloader {
    pub fn new(sha3_code: Vec<u8>, peers_info_table: &PeersInfoTable) -> Self {
        let file_meta_info = peers_info_table
            .file_meta_info_hash_map
            .get(&sha3_code)
            .unwrap()
            .clone();
        let file_piece_owner_table =
            FilePieceOwnerTable::new_from_peers_info_table(&file_meta_info, peers_info_table);
        Self {
            file_meta_info,
            file_piece_owner_table,
        }
    }

    pub async fn start_downloading(
        &self,
        piece_downloaded_buffer: Arc<Mutex<FilePieceDownloadedBuffer>>,
        pi_sender: Sender<P2PToPIMessage>,
        lf_sender: Sender<P2PToLFMessage>,
        finished_signal_sender: oneshot::Sender<()>,
    ) -> Result<(), Box<dyn Error>> {
        let piece_downloaded_buffer_finished_check = piece_downloaded_buffer.clone();
        let (check_sender, mut check_receiver) = mpsc::channel::<()>(15);
        let file_meta_info_check = self.file_meta_info.clone();
        let pieces_number = file_meta_info_check.pieces_info.len();
        // 开启线程监听是否下载结束
        tokio::spawn(async move {
            let mut piece_cnt = 0;
            while let Some(_) = check_receiver.recv().await {
                // 每收到一个信号，都检查一下是不是已经下载完了
                piece_cnt += 1;
                println!("已经接受块：{}， 目标块数： {}", piece_cnt, pieces_number);
                if piece_cnt == pieces_number {
                    // 已经下载完了
                    let mut sorted_piece_sha3_code = vec![Vec::new(); pieces_number];
                    for piece_meta_info in file_meta_info_check.pieces_info.iter() {
                        let index = piece_meta_info.index as usize;
                        assert!(index < pieces_number);
                        sorted_piece_sha3_code[index] = piece_meta_info.sha3_code.clone();
                    }
                    // 按序接收
                    let (file_sender, file_receiver) = oneshot::channel::<tokio::fs::File>();
                    let lf_message = P2PToLFMessage::new_downloaded_file_store_and_update(
                        file_sender,
                        file_meta_info_check.clone(),
                    );
                    lf_sender.send(lf_message).await;
                    let mut file = file_receiver.await.unwrap();
                    for piece_sha3_code in sorted_piece_sha3_code.iter() {
                        let piece_content;
                        {
                            let piece_downloaded_buffer_lock =
                                piece_downloaded_buffer_finished_check.lock().await;
                            piece_content =
                                piece_downloaded_buffer_lock.get_one_piece_content(piece_sha3_code);
                        }
                        let mut pos = 0;
                        let mut data_len = piece_content.len();
                        while pos < data_len {
                            let bytes_written = file.write(&piece_content[pos..]).await.unwrap();
                            pos += bytes_written;
                            println!("bytes_writen: {}", bytes_written);
                        }
                    }
                    println!("文件写入成功");
                    // 删除buffer中的块
                    {
                        let mut piece_downloaded_buffer_lock =
                            piece_downloaded_buffer_finished_check.lock().await;
                        piece_downloaded_buffer_lock
                            .release_file_piece_content(sorted_piece_sha3_code);
                    }
                    // 给pi发信号，文件下载成功了
                    finished_signal_sender.send(());
                    // 记得跳出循环，结束此线程
                    break;
                }
            }
        });
        // 开启线程请求文件块
        println!(
            "发起请求数： {}",
            self.file_piece_owner_table.sorted_piece_request_list.len()
        );
        for piece_request_info in self.file_piece_owner_table.sorted_piece_request_list.iter() {
            let piece_index = piece_request_info.index;
            let piece_sha3_code = piece_request_info.sha3_code.clone();
            let piece_index = piece_request_info.index;
            let socket_addr_vec = self
                .file_piece_owner_table
                .piece_owner_hash_table
                .get(&piece_sha3_code)
                .unwrap()
                .clone();
            let file_sha3_code = self.file_meta_info.sha3_code.clone();
            let file_name = self.file_meta_info.file_name.clone();
            let file_piece_size = self.file_meta_info.file_piece_size;
            let piece_downloaded_buffer_clone = piece_downloaded_buffer.clone();
            let pi_sender_clone = pi_sender.clone();
            let check_sender_clone = check_sender.clone();
            tokio::spawn(async move {
                for socket_addr in socket_addr_vec {
                    if let Ok(mut stream) = TcpStream::connect(socket_addr).await {
                        let mut packet = Vec::<u8>::new();
                        // 发送请求包
                        packet.push(6u8);
                        let piece_request_payload = FilePieceRequestPayload::new(
                            file_sha3_code.clone(),
                            piece_sha3_code.clone(),
                            piece_index,
                            file_piece_size,
                        )
                        .to_bencode()
                        .unwrap();
                        let payload_len = piece_request_payload.len() as u32;
                        packet.extend_from_slice(&payload_len.to_le_bytes().to_vec());
                        packet.extend_from_slice(&piece_request_payload);
                        stream.write(&packet).await;
                        // get File Piece content
                        let mut packet = Vec::new();
                        let mut buf = vec![0u8; (file_piece_size + 10) as usize];
                        let mut is_packet_header_coming = false;
                        let mut packet_expected_length = u32::MAX - 10000;
                        let mut type_id = 29;
                        while let Ok(nbytes) =
                            stream.read(&mut buf[..(file_piece_size as usize)]).await
                        {
                            if is_packet_header_coming {
                                packet.extend_from_slice(&buf[..nbytes]);
                                //println!("get a packet, packet length: {}, current packet length: {}", nbytes, packet.len());
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
                                //println!("get a packet header, type: {}, payload_length: {}, current packet: length: {}", type_id, payload_length, packet.len());
                                if packet_expected_length as usize == packet.len() {
                                    break;
                                }
                            }
                        }
                        match type_id {
                            7u8 => {
                                let piece_content = packet[5..].to_vec();
                                let mut hasher = Sha3_256::new();
                                hasher.update(&piece_content);
                                let expected_piece_sha3_code = hasher.finalize().to_vec();
                                assert_eq!(expected_piece_sha3_code, piece_sha3_code);
                                {
                                    let mut piece_downloaded_buffer_lock =
                                        piece_downloaded_buffer_clone.lock().await;
                                    piece_downloaded_buffer_lock
                                        .add_one_piece(piece_sha3_code.clone(), piece_content);
                                }
                                println!("在buffer中插入块 index {}", piece_index);
                                // TODO 给PI发送一个消息表示自己拿到块了。
                                let file_meta_info_with_empty_piece = FileMetaInfo::new(
                                    file_sha3_code.clone(),
                                    file_name,
                                    file_piece_size,
                                    Vec::new(),
                                );
                                let file_piece_info =
                                    FilePieceInfo::new(piece_index, piece_sha3_code.clone());
                                let message = P2PToPIMessage::new_piece_downloaded_info_message(
                                    file_meta_info_with_empty_piece,
                                    file_piece_info,
                                );
                                pi_sender_clone.send(message).await;
                                // 给checker发消息查看文件快是否已经全部下载了
                                check_sender_clone.send(()).await;
                                // 如果已经下载到了一个块，则退出循环
                                break;
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

pub struct FilePieceOwnerTable {
    sorted_piece_request_list: Vec<FilePieceInfo>,
    piece_owner_hash_table: HashMap<Vec<u8>, Vec<SocketAddr>>,
}

impl FilePieceOwnerTable {
    pub fn new_from_peers_info_table(
        file_meta_info: &FileMetaInfo,
        peers_info_table: &PeersInfoTable,
    ) -> Self {
        let mut piece_owner_hash_table = HashMap::new();
        let mut sorted_piece_request_list = Vec::new();
        for piece_info in file_meta_info.pieces_info.iter() {
            let piece_sha3_code = &piece_info.sha3_code;
            let peer_id_vec = peers_info_table
                .files_piece_info_hash_map
                .get(piece_sha3_code)
                .unwrap();
            sorted_piece_request_list.push((piece_info.clone(), peer_id_vec.len()));
            let mut peer_socket_addr = vec![];
            for peer_id in peer_id_vec {
                peer_socket_addr.push(
                    peers_info_table
                        .peer_socket_addr_hash_map
                        .get(&peer_id)
                        .unwrap()
                        .clone(),
                );
            }
            piece_owner_hash_table.insert(piece_sha3_code.clone(), peer_socket_addr);
        }
        // 排序
        sorted_piece_request_list.sort_by(|a, b| a.1.cmp(&b.1));
        let sorted_piece_request_list = sorted_piece_request_list
            .into_iter()
            .map(|p| p.0)
            .collect::<Vec<FilePieceInfo>>();
        Self {
            sorted_piece_request_list,
            piece_owner_hash_table,
        }
    }
}

impl ToBencode for PeersInfoTable {
    const MAX_DEPTH: usize = 10;
    // // 全局PeersInfo Table
    //     peers_info: Vec<PeerInfo>,
    //     // sha3_code => Peer_id 进行映射，用于全局去重，
    //     files_info_hash_map: HashMap<Vec<u8>, Vec<u32>>, // 全局表, 与本地的表进行同步， //TODO 或许local_file_manager不需要此字段（！！还是需要同步的）
    //     // file_piece_sha3_code => Peer_id
    //     files_piece_info_hash_map: HashMap<Vec<u8>, Vec<u32>>,
    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), bendy::encoding::Error> {
        encoder.emit_dict(|mut e| e.emit_pair(b"peers_info", &self.peers_info))?;
        Ok(())
    }
}

impl FromBencode for PeersInfoTable {
    fn decode_bencode_object(object: Object) -> Result<Self, bendy::decoding::Error> {
        let mut peers_info = None;

        let mut dict = object.try_into_dictionary()?;
        while let Some(pair) = dict.next_pair()? {
            match pair {
                (b"peers_info", value) => {
                    peers_info = Vec::<PeerInfo>::decode_bencode_object(value)
                        .context("peers_info")
                        .map(Some)?;
                }
                (unknown_field, _) => {
                    return Err(bendy::decoding::Error::unexpected_field(
                        String::from_utf8_lossy(unknown_field),
                    ));
                }
            }
        }
        let peers_info =
            peers_info.ok_or_else(|| bendy::decoding::Error::missing_field("peers_info"))?;
        Ok(PeersInfoTable::from_peers_info(peers_info))
    }
}
impl ToBencode for FilePieceInfo {
    const MAX_DEPTH: usize = 3;

    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), bendy::encoding::Error> {
        encoder.emit_dict(|mut e| {
            e.emit_pair(b"index", &self.index)?;
            e.emit_pair(b"sha3_code", &self.sha3_code)
        })?;
        Ok(())
    }
}

impl FromBencode for FilePieceInfo {
    fn decode_bencode_object(object: Object) -> Result<Self, bendy::decoding::Error> {
        let mut index = None;
        let mut sha3_code = None;

        let mut dict = object.try_into_dictionary()?;
        while let Some(pair) = dict.next_pair()? {
            match pair {
                (b"index", value) => {
                    index = u32::decode_bencode_object(value)
                        .context("index")
                        .map(Some)?;
                }
                (b"sha3_code", value) => {
                    sha3_code = Vec::<u8>::decode_bencode_object(value)
                        .context("sha3_code")
                        .map(Some)?;
                }
                (unknown_field, _) => {
                    return Err(bendy::decoding::Error::unexpected_field(
                        String::from_utf8_lossy(unknown_field),
                    ));
                }
            }
        }
        let index = index.ok_or_else(|| bendy::decoding::Error::missing_field("index"))?;
        let sha3_code =
            sha3_code.ok_or_else(|| bendy::decoding::Error::missing_field("sha3_code"))?;
        Ok(FilePieceInfo::new(index, sha3_code))
    }
}

impl ToBencode for FileMetaInfo {
    const MAX_DEPTH: usize = 5;
    //     sha3_code: Vec<u8>,
    //     file_name: OsString,
    //     file_piece_length: u32,
    //     pieces_info: Vec<FilePieceInfo>,
    ///
    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), bendy::encoding::Error> {
        encoder.emit_unsorted_dict(|mut e| {
            e.emit_pair(b"sha3_code", self.sha3_code.as_slice())?;
            e.emit_pair(b"file_name", self.file_name.to_str().unwrap())?;
            e.emit_pair(b"file_piece_size", self.file_piece_size)?;
            e.emit_pair(b"pieces_info", &self.pieces_info)
        })?;
        Ok(())
    }
}

impl FromBencode for FileMetaInfo {
    const EXPECTED_RECURSION_DEPTH: usize = 5;
    fn decode_bencode_object(object: Object) -> Result<Self, bendy::decoding::Error> {
        let mut sha3_code = None;
        let mut file_name = None;
        let mut file_piece_size = None;
        let mut pieces_info = None;

        let mut dict = object.try_into_dictionary()?;
        while let Some(pair) = dict.next_pair()? {
            match pair {
                (b"sha3_code", value) => {
                    sha3_code = Vec::<u8>::decode_bencode_object(value)
                        .context("sha3_code")
                        .map(Some)?;
                }
                (b"file_name", value) => {
                    file_name = String::decode_bencode_object(value)
                        .context("file_name")
                        .map(Some)?;
                }
                (b"file_piece_size", value) => {
                    file_piece_size = u32::decode_bencode_object(value)
                        .context("file_piece_size")
                        .map(Some)?;
                }
                (b"pieces_info", value) => {
                    pieces_info = Vec::<FilePieceInfo>::decode_bencode_object(value)
                        .context("pieces_info")
                        .map(Some)?;
                }
                (unknown_field, _) => {
                    return Err(bendy::decoding::Error::unexpected_field(
                        String::from_utf8_lossy(unknown_field),
                    ));
                }
            }
        }
        let sha3_code =
            sha3_code.ok_or_else(|| bendy::decoding::Error::missing_field("sha3_code"))?;
        let file_name =
            file_name.ok_or_else(|| bendy::decoding::Error::missing_field("file_name"))?;
        let file_name = OsString::from(file_name);
        let file_piece_size = file_piece_size
            .ok_or_else(|| bendy::decoding::Error::missing_field("file_piece_size"))?;
        let pieces_info =
            pieces_info.ok_or_else(|| bendy::decoding::Error::missing_field("pieces_info"))?;
        Ok(FileMetaInfo::new(
            sha3_code,
            file_name,
            file_piece_size,
            pieces_info,
        ))
    }
}

impl ToBencode for PeerInfo {
    const MAX_DEPTH: usize = 7;
    ///     peer_id: u32,
    //     peer_ip: IpAddr,
    //     peer_open_port: u16,
    //     peer_file_meta_info_report: Vec<FileMetaInfo>,
    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), bendy::encoding::Error> {
        encoder.emit_unsorted_dict(|e| {
            e.emit_pair(b"peer_id", &self.peer_id)?;
            e.emit_pair(b"peer_ip", &self.peer_ip)?;
            e.emit_pair(b"peer_open_port", &self.peer_open_port)?;
            e.emit_pair(
                b"peer_file_meta_info_report",
                &self.peer_file_meta_info_report,
            )
        })?;
        Ok(())
    }
}

impl FromBencode for PeerInfo {
    ///     peer_id: u32,
    //     peer_ip: Vec<u8>,
    //     peer_open_port: u16,
    //     peer_file_meta_info_report: Vec<FileMetaInfo>,
    fn decode_bencode_object(object: Object) -> Result<Self, bendy::decoding::Error> {
        let mut peer_id = None;
        let mut peer_ip = None;
        let mut peer_open_port = None;
        let mut peer_file_meta_info_report = None;
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
                (b"peer_open_port", value) => {
                    peer_open_port = u16::decode_bencode_object(value)
                        .context("peer_open_port")
                        .map(Some)?;
                }
                (b"peer_file_meta_info_report", value) => {
                    peer_file_meta_info_report = Vec::<FileMetaInfo>::decode_bencode_object(value)
                        .context("peer_file_meta_info_report")
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
        let peer_open_port = peer_open_port
            .ok_or_else(|| bendy::decoding::Error::missing_field("peer_open_port"))?;
        let peer_file_meta_info_report = peer_file_meta_info_report
            .ok_or_else(|| bendy::decoding::Error::missing_field("peer_file_meta_info_report"))?;
        Ok(PeerInfo::new(
            peer_id,
            peer_ip,
            peer_open_port,
            peer_file_meta_info_report,
        ))
    }
}

#[cfg(test)]
mod test_bencode {
    use super::{FileMetaInfo, FilePieceInfo, PeerInfo};
    use bendy::decoding::FromBencode;
    use bendy::encoding::ToBencode;
    use std::ffi::OsString;
    #[test]
    fn test_file_meta_info_with_benecode() {
        let sha3_code = (0..32).collect::<Vec<u8>>();
        let file_piece_info = FilePieceInfo::new(12, sha3_code);
        let encoded = file_piece_info.to_bencode().unwrap();
        let expected_file_piece_info = FilePieceInfo::from_bencode(&encoded).unwrap();
        assert_eq!(file_piece_info, expected_file_piece_info);
    }

    #[test]
    fn test_file_piece_info_with_bencode() {
        let sha3_code = (0..32).collect::<Vec<u8>>();
        let file_piece_info = FilePieceInfo::new(12, sha3_code.clone());
        let file_meta_info = FileMetaInfo::new(
            sha3_code,
            OsString::from("luweiba.pdf"),
            1 << 16,
            vec![file_piece_info],
        );
        let encoded = file_meta_info.to_bencode().unwrap();
        let expected_file_meta_info = FileMetaInfo::from_bencode(&encoded).unwrap();
        assert_eq!(expected_file_meta_info, file_meta_info);
    }

    #[test]
    fn test_peer_info_with_bencode() {
        let sha3_code = (0..32).collect::<Vec<u8>>();
        let file_piece_info = FilePieceInfo::new(12, sha3_code.clone());
        let file_meta_info = FileMetaInfo::new(
            sha3_code,
            OsString::from("luweiba.pdf"),
            1 << 16,
            vec![file_piece_info],
        );
        let peer_info = PeerInfo::new(16, vec![127, 0, 0, 1], 8080, vec![file_meta_info]);
        let encoded = peer_info.to_bencode().unwrap();
        let expected_peer_info = PeerInfo::from_bencode(&encoded).unwrap();
        assert_eq!(expected_peer_info, peer_info);
    }
}
