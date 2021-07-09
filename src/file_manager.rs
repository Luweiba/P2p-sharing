use crate::thread_communication_message::{LFToPIMessage, P2PToLFMessage};
use crate::types::{FileMetaInfo, FilePieceInfo};
use bendy::encoding::ToBencode;
use sha3::{Digest, Sha3_256};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::fs::{DirEntry, File};
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;
use tokio::time::Duration;

const BUF_SIZE: usize = 1 << 18;

#[derive(Debug)]
pub struct LocalFileTable {
    /// local directory to share
    base_dir: PathBuf,
    /// 分片大小
    piece_size: u32,
    /// 本地文件的 哈希值 => path（用于检测相同文件）
    files_info_hash_map: HashMap<Vec<u8>, PathBuf>,
    /// 存储本地文件信息
    file_meta_info_report: Vec<FileMetaInfo>,
    /// 存储本地文件路径名
    file_path_set: HashSet<PathBuf>,
}

pub struct LocalFileManager {
    local_file_table: Arc<Mutex<LocalFileTable>>,
}

impl LocalFileManager {
    pub async fn new_and_initialize(
        base_dir: PathBuf,
        piece_size: u32,
    ) -> Result<Self, Box<dyn Error>> {
        let mut local_file_table = LocalFileTable::new(base_dir, piece_size);
        local_file_table.initialize().await?;
        Ok(Self {
            local_file_table: Arc::new(Mutex::new(local_file_table)),
        })
    }
    pub async fn get_file_meta_info_report(&self) -> Vec<FileMetaInfo> {
        let local_file_table_lock = self.local_file_table.lock().await;
        local_file_table_lock.get_file_meta_info_report()
    }
    pub async fn start_peer_to_peer_service(&self, mut p2p_receiver: Receiver<P2PToLFMessage>) {
        let local_file_table_clone = self.local_file_table.clone();
        tokio::spawn(async move {
            //println!("开启本地文件manager与P2P的通信服务");
            loop {
                if let Some(message) = p2p_receiver.recv().await {
                    match message {
                        P2PToLFMessage::GetLocalPath {
                            file_sha3_code,
                            sender,
                        } => {
                            let local_file_table_lock = local_file_table_clone.lock().await;
                            if let Some(path) = local_file_table_lock
                                .files_info_hash_map
                                .get(&file_sha3_code)
                            {
                                sender.send(path.clone());
                            }
                        }
                        P2PToLFMessage::DownloadedFileStoreAndUpdate {
                            sender,
                            file_meta_info,
                        } => {
                            // 更新本地信息并创建一个文件，此更新信息不必发送给PI用于广播（因为piece广播已经添加了）
                            let local_base_dir;
                            {
                                let local_file_table_lock = local_file_table_clone.lock().await;
                                local_base_dir = local_file_table_lock.base_dir.clone();
                            }
                            // 创建文件
                            let file_name = file_meta_info.get_file_name();
                            let new_file_path = local_base_dir.join(file_name);
                            // TODO 假设不同内容的文件不会重名
                            let new_file = tokio::fs::File::create(&new_file_path).await.unwrap();
                            // 给file_downloader发过去创建的文件
                            sender.send(new_file);
                            // 更新本地的local_file_table
                            let file_sha3_code = file_meta_info.get_file_sha3_code();
                            {
                                let mut local_file_table_lock = local_file_table_clone.lock().await;
                                local_file_table_lock
                                    .file_meta_info_report
                                    .push(file_meta_info);
                                local_file_table_lock
                                    .files_info_hash_map
                                    .insert(file_sha3_code, new_file_path.clone());
                                local_file_table_lock.file_path_set.insert(new_file_path);
                            }
                        }
                    }
                }
            }
        });
    }
    pub async fn start_scanning_and_updating_periodically(
        &mut self,
        scanning_interval: u64,
        pi_sender: Sender<LFToPIMessage>,
        mut pi_receiver: Receiver<LFToPIMessage>,
    ) -> Result<(), Box<dyn Error>> {
        // 接收来自PeerInfo的通信来修改本地文件信息
        tokio::spawn(async move {
            while let Some(message) = pi_receiver.recv().await {
                // TODO do updating
            }
        });
        // 定时扫描本地文件信息并发送更新信息
        let local_file_table_clone = self.local_file_table.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(scanning_interval)).await;
                //println!("Start Scanning and updating ...");
                {
                    let mut local_file_table_lock = local_file_table_clone.lock().await;
                    local_file_table_lock
                        .scan_and_update(pi_sender.clone())
                        .await;
                }
            }
        });
        Ok(())
    }
}

impl LocalFileTable {
    pub fn new(base_dir: PathBuf, piece_size: u32) -> Self {
        Self {
            base_dir,
            piece_size,
            files_info_hash_map: HashMap::new(),
            file_meta_info_report: Vec::new(),
            file_path_set: HashSet::new(),
        }
    }

    pub async fn initialize(&mut self) -> Result<(), Box<dyn Error>> {
        let mut entries = fs::read_dir(&self.base_dir).await?;
        let mut buf = vec![0u8; BUF_SIZE];
        while let Some(entry) = entries.next_entry().await? {
            if let Some(file_meta_info) = self.handle_entry(entry, &mut buf).await? {
                self.file_meta_info_report.push(file_meta_info);
            }
        }
        //println!("Local File manager initialization finished.");
        Ok(())
    }
    /// 扫描本地文件查看更新信息
    /// TODO
    async fn scan_and_update(
        &mut self,
        pi_sender: Sender<LFToPIMessage>,
    ) -> Result<(), Box<dyn Error>> {
        let mut entries = tokio::fs::read_dir(&self.base_dir).await?;
        let mut lf_to_pi_message = None;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let metadata = entry.metadata().await?;
            if !metadata.is_file() {
                continue;
            }
            // 检测到新文件
            if !self.file_path_set.contains(&path) {
                let mut buf = vec![0u8; self.piece_size as usize];
                let mut file = File::open(entry.path()).await?;
                let file_name = entry.file_name();
                let mut file_hasher = Sha3_256::new();
                let mut index = 0;
                let mut pieces_info = vec![];
                let mut piece_buffer = Vec::with_capacity(self.piece_size as usize);
                while let Ok(nbytes) = file.read(&mut buf[..]).await {
                    if nbytes == 0 {
                        if piece_buffer.len() != 0 {
                            // 计算哈希值
                            file_hasher.update(&piece_buffer[..]);
                            let mut piece_hasher = Sha3_256::new();
                            piece_hasher.update(&piece_buffer[..]);
                            let piece_sha3_code = piece_hasher.finalize().to_vec();
                            pieces_info.push(FilePieceInfo::new(index, piece_sha3_code.into()));
                            index += 1;
                            piece_buffer.clear();
                        }
                        //println!("读取结束");
                        break;
                    }
                    if piece_buffer.len() + nbytes > self.piece_size as usize {
                        let gap = self.piece_size as usize - piece_buffer.len();
                        piece_buffer.extend_from_slice(&buf[..gap]);
                        // 计算哈希值
                        file_hasher.update(&piece_buffer[..]);
                        let mut piece_hasher = Sha3_256::new();
                        piece_hasher.update(&piece_buffer[..]);
                        let piece_sha3_code = piece_hasher.finalize().to_vec();
                        pieces_info.push(FilePieceInfo::new(index, piece_sha3_code.into()));
                        index += 1;
                        piece_buffer.clear();
                        //println!("piece_buffer len: {}", piece_buffer.len());
                        piece_buffer.extend_from_slice(&buf[gap..]);
                    } else {
                        piece_buffer.extend_from_slice(&buf[..nbytes]);
                        //println!("piece_buffer len: {}", piece_buffer.len());
                        if piece_buffer.len() == self.piece_size as usize {
                            // 计算哈希值
                            file_hasher.update(&piece_buffer[..]);
                            let mut piece_hasher = Sha3_256::new();
                            piece_hasher.update(&piece_buffer[..]);
                            let piece_sha3_code = piece_hasher.finalize().to_vec();
                            pieces_info.push(FilePieceInfo::new(index, piece_sha3_code.into()));
                            index += 1;
                            piece_buffer.clear();
                            //println!("piece_buffer len: {}", piece_buffer.len());
                        }
                    }
                }
                let file_sha3_code = file_hasher.finalize().to_vec();
                if !self.files_info_hash_map.contains_key(&file_sha3_code) {
                    self.files_info_hash_map
                        .insert(file_sha3_code.clone(), entry.path());
                    let file_meta_info = FileMetaInfo::new(
                        file_sha3_code.clone(),
                        file_name,
                        self.piece_size,
                        pieces_info,
                    );
                    self.file_meta_info_report.push(file_meta_info.clone());
                    self.file_path_set.insert(entry.path());
                    if lf_to_pi_message.is_none() {
                        let mut message = LFToPIMessage::new();
                        message.add_file_meta_info(file_meta_info);
                        lf_to_pi_message = Some(message);
                    } else {
                        let message = lf_to_pi_message.as_mut().unwrap();
                        message.add_file_meta_info(file_meta_info);
                    }
                }
            }
        }
        if let Some(message) = lf_to_pi_message {
            pi_sender.send(message).await?;
        }
        Ok(())
    }

    pub fn get_file_meta_info_report(&self) -> Vec<FileMetaInfo> {
        self.file_meta_info_report
            .iter()
            .map(|item| item.clone())
            .collect()
    }

    async fn handle_entry(
        &mut self,
        entry: DirEntry,
        buf: &mut Vec<u8>,
    ) -> Result<Option<FileMetaInfo>, Box<dyn Error>> {
        let metadata = entry.metadata().await?;
        if metadata.is_file() {
            let mut file = File::open(entry.path()).await?;
            let file_name = entry.file_name();
            let mut file_hasher = Sha3_256::new();
            let mut index = 0;
            let mut pieces_info = vec![];
            let mut piece_buffer = Vec::with_capacity(self.piece_size as usize);
            while let Ok(nbytes) = file.read(&mut buf[..]).await {
                if nbytes == 0 {
                    if piece_buffer.len() != 0 {
                        // 计算哈希值
                        file_hasher.update(&piece_buffer[..]);
                        let mut piece_hasher = Sha3_256::new();
                        piece_hasher.update(&piece_buffer[..]);
                        let piece_sha3_code = piece_hasher.finalize().to_vec();
                        pieces_info.push(FilePieceInfo::new(index, piece_sha3_code.into()));
                        index += 1;
                        piece_buffer.clear();
                        //println!("piece_buffer len: {}", piece_buffer.len());
                    }
                    // println!("读取结束");
                    break;
                }
                if piece_buffer.len() + nbytes > self.piece_size as usize {
                    let gap = self.piece_size as usize - piece_buffer.len();
                    piece_buffer.extend_from_slice(&buf[..gap]);
                    // 计算哈希值
                    file_hasher.update(&piece_buffer[..]);
                    let mut piece_hasher = Sha3_256::new();
                    piece_hasher.update(&piece_buffer[..]);
                    let piece_sha3_code = piece_hasher.finalize().to_vec();
                    pieces_info.push(FilePieceInfo::new(index, piece_sha3_code.into()));
                    index += 1;
                    piece_buffer.clear();
                    //println!("piece_buffer len: {}", piece_buffer.len());
                    piece_buffer.extend_from_slice(&buf[gap..]);
                } else {
                    piece_buffer.extend_from_slice(&buf[..nbytes]);
                    //println!("piece_buffer len: {}", piece_buffer.len());
                    if piece_buffer.len() == self.piece_size as usize {
                        // 计算哈希值
                        file_hasher.update(&piece_buffer[..]);
                        let mut piece_hasher = Sha3_256::new();
                        piece_hasher.update(&piece_buffer[..]);
                        let piece_sha3_code = piece_hasher.finalize().to_vec();
                        pieces_info.push(FilePieceInfo::new(index, piece_sha3_code.into()));
                        index += 1;
                        piece_buffer.clear();
                        //println!("piece_buffer len: {}", piece_buffer.len());
                    }
                }
            }
            let file_sha3_code = file_hasher.finalize().to_vec();
            if !self.files_info_hash_map.contains_key(&file_sha3_code) {
                let file_path = entry.path();
                self.file_path_set.insert(entry.path());
                self.files_info_hash_map
                    .insert(file_sha3_code.clone(), file_path);
                Ok(Some(FileMetaInfo::new(
                    file_sha3_code,
                    file_name,
                    self.piece_size,
                    pieces_info,
                )))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}
