use crate::message::FilePieceRequestPayload;
use crate::thread_communication_message::{P2PToLFMessage, P2PToPIMessage};
use crate::types::{FileDownloader, FileMetaInfo, PeersInfoTable};
use bendy::decoding::FromBencode;
use sha3::{Digest, Sha3_256};
use std::collections::HashMap;
use std::error::Error;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom};
use tokio::net::TcpListener;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{oneshot, Mutex};

const BUF_SIZE: usize = 1 << 18;
/// 用于管理对等方的通信
/// 主要包括：请求文件和应答文件
pub struct PeerToPeerManager {
    // 本机用于Peer to Peer的端口号
    open_port: u16,
    // 本机绑定IP
    local_bind_ip: Ipv4Addr,
}

impl PeerToPeerManager {
    pub fn new(local_bind_ip: Ipv4Addr, open_port: u16) -> Self {
        Self {
            open_port,
            local_bind_ip,
        }
    }

    pub async fn start_distributing_and_downloading(
        &mut self,
        pi_sender: Sender<P2PToPIMessage>,
        mut pi_receiver: Receiver<P2PToPIMessage>,
        lf_sender: Sender<P2PToLFMessage>,
    ) -> Result<(), Box<dyn Error>> {
        log::info!("[P2P]: 开启内容分发与下载服务...");
        // piece_sha3_code => piece_content_in_bytes
        let piece_downloaded_buffer = Arc::new(Mutex::new(FilePieceDownloadedBuffer::new()));
        let p2p_listener = TcpListener::bind(SocketAddr::from(SocketAddrV4::new(
            self.local_bind_ip,
            self.open_port,
        )))
        .await?;
        // 开启P2P分发端口
        let distributing_piece_downloaded_buffer = piece_downloaded_buffer.clone();
        let lf_sender_clone = lf_sender.clone();
        tokio::spawn(async move {
            let mut buf = vec![0u8; BUF_SIZE];
            loop {
                if let Ok((mut stream, peer_addr)) = p2p_listener.accept().await {
                    // TODO
                    let mut type_id = 29;
                    let mut packet = Vec::new();
                    let mut is_packet_header_coming = false;
                    let mut packet_expected_length = u32::MAX - 10000;
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
                        6u8 => {
                            // FilePieceRequestPayload
                            let file_piece_request_payload =
                                FilePieceRequestPayload::from_bencode(&packet[5..]).unwrap();
                            let (
                                file_sha3_code,
                                file_piece_sha3_code,
                                file_piece_index,
                                file_piece_size,
                            ) = file_piece_request_payload.get_inner_info();
                            // 查看是不是缓存在buffer中
                            let mut is_in_buffer;
                            {
                                let piece_downloaded_buffer_lock =
                                    distributing_piece_downloaded_buffer.lock().await;
                                is_in_buffer = piece_downloaded_buffer_lock
                                    .file_piece_downloaded_buf_map
                                    .contains_key(&file_piece_sha3_code);
                            }
                            if is_in_buffer {
                                log::debug!("[P2P]: 请求的块在buffer中...");
                                let piece_content;
                                {
                                    let piece_downloaded_buffer_lock =
                                        distributing_piece_downloaded_buffer.lock().await;
                                    piece_content = piece_downloaded_buffer_lock
                                        .get_one_piece_content(&file_piece_sha3_code);
                                }
                                stream.write(&piece_content).await;
                            } else {
                                // 块存储在本地中
                                // 向PeersInfoManager请求PeersInfoTable的一份拷贝
                                let (sender, path_receiver) = oneshot::channel::<PathBuf>();
                                let get_local_path_message =
                                    P2PToLFMessage::new_get_local_path(file_sha3_code, sender);
                                // 等待本地文件路径并校验后分发块
                                tokio::spawn(async move {
                                    // 等待本地文件路径
                                    let local_path = path_receiver.await.unwrap();
                                    let mut file = File::open(local_path).await.unwrap();
                                    let offset = file_piece_size * file_piece_index;
                                    let mut buf = vec![0u8; (file_piece_size + 10) as usize];
                                    let mut piece_content =
                                        Vec::with_capacity(file_piece_size as usize);
                                    file.seek(SeekFrom::Start(offset as u64)).await;
                                    while let Ok(nbytes) =
                                        file.read(&mut buf[..(file_piece_size as usize)]).await
                                    {
                                        if nbytes == 0 {
                                            break;
                                        }
                                        if piece_content.len() + nbytes > file_piece_size as usize {
                                            piece_content.extend_from_slice(
                                                &buf[..(file_piece_size as usize
                                                    - piece_content.len())],
                                            );
                                            break;
                                        } else {
                                            piece_content.extend_from_slice(&buf[..nbytes]);
                                            if piece_content.len() == file_piece_size as usize {
                                                break;
                                            }
                                        }
                                    }
                                    let mut hasher = Sha3_256::new();
                                    hasher.update(&piece_content[..]);
                                    let piece_sha3_code = hasher.finalize().to_vec();
                                    assert_eq!(piece_sha3_code, file_piece_sha3_code);
                                    let mut packet = Vec::new();
                                    packet.push(7u8);
                                    let piece_content_len = piece_content.len() as u32;
                                    packet.extend_from_slice(
                                        &piece_content_len.to_le_bytes().to_vec(),
                                    );
                                    packet.extend_from_slice(&piece_content[..]);
                                    stream.write(&packet[..]).await;
                                });
                                lf_sender_clone.send(get_local_path_message).await;
                            }
                        }
                        _ => {}
                    }
                }
            }
        });
        // P2P downloading
        let downloading_piece_downloaded_buffer = piece_downloaded_buffer.clone();
        let downloading_pi_sender = pi_sender.clone();
        let downloading_lf_sender = lf_sender.clone();
        // 从peerInfoManager获取要求下载的列表
        tokio::spawn(async move {
            loop {
                if let Some(message) = pi_receiver.recv().await {
                    match message {
                        P2PToPIMessage::DownloadedFileInfo {
                            peers_info_table_snapshot,
                            file_download_sha3_code,
                            sender, // 告知已经下载结束
                        } => {
                            let file_meta_info = peers_info_table_snapshot
                                .get_file_meta_info_from_sha3_code(&file_download_sha3_code)
                                .unwrap();
                            log::info!(
                                "[P2P]: 开始下载: {:?}, 文件分片大小: {}, 文件分片数: {} ",
                                file_meta_info.get_file_name(),
                                file_meta_info.get_file_piece_size(),
                                file_meta_info.get_file_piece_number()
                            );
                            let file_downloader = FileDownloader::new(
                                file_download_sha3_code,
                                &peers_info_table_snapshot,
                            );
                            file_downloader
                                .start_downloading(
                                    downloading_piece_downloaded_buffer.clone(),
                                    downloading_pi_sender.clone(),
                                    downloading_lf_sender.clone(),
                                    sender,
                                )
                                .await;
                        }
                        _ => {}
                    }
                }
            }
        });
        Ok(())
    }

    pub fn get_open_port(&self) -> u16 {
        self.open_port
    }
}

pub struct FilePieceDownloadedBuffer {
    file_piece_downloaded_buf_map: HashMap<Vec<u8>, Vec<u8>>,
}

impl FilePieceDownloadedBuffer {
    pub fn new() -> Self {
        Self {
            file_piece_downloaded_buf_map: HashMap::new(),
        }
    }

    pub fn add_one_piece(&mut self, piece_sha3_code: Vec<u8>, piece_content: Vec<u8>) {
        self.file_piece_downloaded_buf_map
            .insert(piece_sha3_code, piece_content);
    }

    pub fn get_one_piece_content(&self, piece_sha3_code: &Vec<u8>) -> Vec<u8> {
        self.file_piece_downloaded_buf_map
            .get(piece_sha3_code)
            .unwrap()
            .clone()
    }

    pub fn release_file_piece_content(&mut self, piece_sha3_code_list: Vec<Vec<u8>>) {
        for piece_sha3_code in piece_sha3_code_list {
            self.file_piece_downloaded_buf_map.remove(&piece_sha3_code);
        }
        log::debug!("[P2P]: 缓存中的文件块释放成功");
    }
}
