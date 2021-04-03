use bendy::decoding::{FromBencode, Object, ResultExt};
use bendy::encoding::{Error, SingleItemEncoder, ToBencode};
use std::collections::HashMap;
use std::ffi::OsString;
use std::net::{SocketAddr, SocketAddrV4};

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
    // sha3_code => Peer_id 进行映射，用于全局去重，
    files_info_hash_map: HashMap<Vec<u8>, Vec<u32>>, // 全局表, 与本地的表进行同步， //TODO 或许local_file_manager不需要此字段（！！还是需要同步的）
    // file_piece_sha3_code => Peer_id
    files_piece_info_hash_map: HashMap<Vec<u8>, Vec<u32>>,
}

impl PeersInfoTable {
    pub fn new() -> Self {
        Self {
            peers_info: Vec::new(),
            files_info_hash_map: HashMap::new(),
            files_piece_info_hash_map: HashMap::new(),
        }
    }

    pub fn from_peers_info(peers_info: Vec<PeerInfo>) -> Self {
        let mut files_info_hash_map = HashMap::new();
        let mut files_piece_info_hash_map = HashMap::new();
        for peer_info in peers_info.iter() {
            let peer_id = peer_info.peer_id;
            let peer_ip = peer_info.peer_ip.clone();
            let peer_open_port = peer_info.peer_open_port;
            for file_meta_info in peer_info.peer_file_meta_info_report.iter() {
                let file_sha3_code = file_meta_info.sha3_code.clone();
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
            files_info_hash_map,
            files_piece_info_hash_map,
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
            let sha3_code = file_meta_info.sha3_code;
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

    // 处理接收的peerInfo信息
    pub fn add_one_peer_info(&mut self, peer_info: PeerInfo) {
        let peer_id = peer_info.peer_id;
        let peer_ip = peer_info.peer_ip.clone();
        let peer_open_port = peer_info.peer_open_port;
        let peer_file_meta_info_report = peer_info.peer_file_meta_info_report.clone();
        for meta_info in peer_file_meta_info_report {
            // 处理文件去重信息
            let sha3_code = meta_info.sha3_code;
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
                    let mut dropped_index= file_peer_id_vec.len();
                    for (index, peer_id) in file_peer_id_vec.iter().enumerate() {
                        if *peer_id == dropped_peer_id {
                            dropped_index = index;
                            break;
                        }
                    }
                    if dropped_index < file_peer_id_vec.len() {
                        file_peer_id_vec.remove(dropped_index);
                        //println!("删除file_peer_id {}", dropped_peer_id);
                        if file_peer_id_vec.is_empty() {
                            delete_pair_flag = true;
                        }
                    }
                }
                if delete_pair_flag {
                    self.files_info_hash_map.remove(file_sha3_code);
                }
                // 处理file_piece的Hash表中的Peer信息
                let file_piece_meta_info_report = &file_meta_info.pieces_info;
                for file_piece_meta_info in file_piece_meta_info_report {
                    let file_piece_sha3_code = &file_piece_meta_info.sha3_code;
                    let mut delete_pair_flag = false;
                    if let Some(file_piece_peer_id_vec) = self.files_piece_info_hash_map.get_mut(file_piece_sha3_code) {
                        let mut dropped_index = file_piece_peer_id_vec.len();
                        for (index, peer_id) in file_piece_peer_id_vec.iter().enumerate() {
                            if *peer_id == dropped_peer_id {
                                dropped_index = index;
                                break;
                            }
                        }
                        if dropped_index < file_piece_peer_id_vec.len() {
                            file_piece_peer_id_vec.remove(dropped_index);
                            //println!("Piece删除file_peer_id {}", dropped_peer_id);
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
