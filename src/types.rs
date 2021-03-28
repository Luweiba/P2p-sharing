use uuid::Uuid;
use std::ffi::OsString;
use bendy::encoding::{ToBencode, SingleItemEncoder};
use bendy::decoding::{FromBencode, Object, ResultExt};


#[derive(Debug, Eq, PartialEq)]
pub struct FileMetaInfo {
    uuid: Uuid,
    sha3_code: Vec<u8>,
    file_name: OsString,
    file_piece_size: u32,
    pieces_info: Vec<FilePieceInfo>,
}

impl FileMetaInfo {
    pub fn new(uuid: Uuid, sha3_code: Vec<u8>, file_name: OsString, file_piece_size: u32, pieces_info: Vec<FilePieceInfo>) -> Self {
        Self {
            uuid,
            sha3_code,
            file_name,
            file_piece_size,
            pieces_info,
        }
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct FilePieceInfo {
    index: u32,
    sha3_code: Vec<u8>,
}

impl FilePieceInfo {
    pub fn new(index: u32, sha3_code: Vec<u8>) -> Self {
        Self {
            index,
            sha3_code,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct PeerInfo {
    peer_id: u32,
    peer_ip: Vec<u8>,
    peer_open_port: u16,
    peer_file_meta_info_report: Vec<FileMetaInfo>,
}

impl PeerInfo {
    pub fn new(peer_id: u32, peer_ip: Vec<u8>, peer_open_port: u16, peer_file_meta_info_report: Vec<FileMetaInfo>) -> Self{
        assert_eq!(peer_ip.len(), 4);
        Self {
            peer_id,
            peer_ip,
            peer_open_port,
            peer_file_meta_info_report
        }
    }
}

#[derive(Debug)]
pub struct PeersInfoTable {
    peers_info: Vec<PeerInfo>,
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
                },
                (b"sha3_code", value) => {
                    sha3_code = Vec::<u8>::decode_bencode_object(value)
                        .context("sha3_code")
                        .map(Some)?;
                },
                (unknown_field, _) => {
                    return Err(bendy::decoding::Error::unexpected_field(String::from_utf8_lossy(
                        unknown_field,
                    )));
                },
            }
        }
        let index = index.ok_or_else(|| bendy::decoding::Error::missing_field("index"))?;
        let sha3_code= sha3_code.ok_or_else(|| bendy::decoding::Error::missing_field("sha3_code"))?;
        Ok(FilePieceInfo::new(index, sha3_code))
    }
}

impl ToBencode for FileMetaInfo {
    const MAX_DEPTH: usize = 5;
    ///     uuid: Uuid,
    //     sha3_code: Vec<u8>,
    //     file_name: OsString,
    //     file_piece_length: u32,
    //     pieces_info: Vec<FilePieceInfo>,
    ///
    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), bendy::encoding::Error> {
        encoder.emit_unsorted_dict(|mut e| {
            e.emit_pair(b"uuid", self.uuid.as_bytes().to_vec())?;
            e.emit_pair(b"sha3_code", self.sha3_code.as_slice())?;
            e.emit_pair(b"file_name", self.file_name.to_str().unwrap())?;
            e.emit_pair(b"file_piece_size", self.file_piece_size)?;
            e.emit_pair(b"pieces_info", &self.pieces_info)
        })?;
        Ok(())
        // d9:file_name11:luweiba.pdf15:file_piece_sizei65536e11:pieces_infold5:indexi12e9:sha3_codeli0ei1ei2ei3ei4ei5ei6ei7ei8ei9ei10ei11ei12ei13ei14ei15ei16ei17ei18ei19ei20ei21ei22ei2
        // 3ei24ei25ei26ei27ei28ei29ei30ei31eeee9:sha3_codeli0ei1ei2ei3ei4ei5ei6ei7ei8ei9ei10ei11ei12ei13ei14ei15ei16ei17ei18ei19ei20ei21ei22ei23ei24ei25ei26ei27ei28ei29ei30ei31ee4:uuidli38ei132e
        // i249ei134ei78ei35ei78ei36ei147ei205ei58ei69ei58ei143ei181ei200eee
    }
}

impl FromBencode for FileMetaInfo {
    const EXPECTED_RECURSION_DEPTH: usize = 5;
    fn decode_bencode_object(object: Object) -> Result<Self, bendy::decoding::Error>  {
        let mut uuid = None;
        let mut sha3_code = None;
        let mut file_name = None;
        let mut file_piece_size = None;
        let mut pieces_info = None;

        let mut dict = object.try_into_dictionary()?;
        while let Some(pair) = dict.next_pair()? {
           match pair {
               (b"uuid", value) => {
                   uuid = Vec::<u8>::decode_bencode_object(value)
                       .context("uuid")
                       .map(Some)?;
               },
               (b"sha3_code", value) => {
                   sha3_code = Vec::<u8>::decode_bencode_object(value)
                       .context("sha3_code")
                       .map(Some)?;
               },
               (b"file_name", value) => {
                   file_name = String::decode_bencode_object(value)
                       .context("file_name")
                       .map(Some)?;
               },
               (b"file_piece_size", value) => {
                   file_piece_size = u32::decode_bencode_object(value)
                       .context("file_piece_size")
                       .map(Some)?;
               },
               (b"pieces_info", value) => {
                   pieces_info = Vec::<FilePieceInfo>::decode_bencode_object(value)
                       .context("pieces_info")
                       .map(Some)?;
               },
               (unknown_field, _) => {
                   return Err(bendy::decoding::Error::unexpected_field(String::from_utf8_lossy(
                       unknown_field,
                   )));
               },
           }
        }
        let uuid = uuid.ok_or_else(|| bendy::decoding::Error::missing_field("uuid"))?;
        let uuid = Uuid::from_slice(&uuid)?;
        let sha3_code = sha3_code.ok_or_else(|| bendy::decoding::Error::missing_field("sha3_code"))?;
        let file_name = file_name.ok_or_else(|| bendy::decoding::Error::missing_field("file_name"))?;
        let file_name = OsString::from(file_name);
        let file_piece_size = file_piece_size.ok_or_else(|| bendy::decoding::Error::missing_field("file_piece_size"))?;
        let pieces_info = pieces_info.ok_or_else(|| bendy::decoding::Error::missing_field("pieces_info"))?;
        Ok(FileMetaInfo::new(uuid, sha3_code, file_name, file_piece_size, pieces_info))
    }
}

impl ToBencode for PeerInfo {
    const MAX_DEPTH: usize = 7;
    ///     peer_id: u32,
    //     peer_ip: IpAddr,
    //     peer_open_port: u16,
    //     peer_file_meta_info_report: Vec<FileMetaInfo>,
    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), bendy::encoding::Error> {
        encoder.emit_unsorted_dict(|mut e| {
            e.emit_pair(b"peer_id", &self.peer_id)?;
            e.emit_pair(b"peer_ip", &self.peer_ip)?;
            e.emit_pair(b"peer_open_port", &self.peer_open_port)?;
            e.emit_pair(b"peer_file_meta_info_report", &self.peer_file_meta_info_report)
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
                },
                (b"peer_ip", value) => {
                    peer_ip = Vec::<u8>::decode_bencode_object(value)
                        .context("peer_ip")
                        .map(Some)?;
                },
                (b"peer_open_port", value) => {
                    peer_open_port = u16::decode_bencode_object(value)
                        .context("peer_open_port")
                        .map(Some)?;
                },
                (b"peer_file_meta_info_report", value) => {
                    peer_file_meta_info_report = Vec::<FileMetaInfo>::decode_bencode_object(value)
                        .context("peer_file_meta_info_report")
                        .map(Some)?;
                },
                (unknown_field, _) => {
                    return Err(bendy::decoding::Error::unexpected_field(String::from_utf8_lossy(
                        unknown_field,
                    )));
                },
            }
        }
        let peer_id = peer_id.ok_or_else(|| bendy::decoding::Error::missing_field("peer_id"))?;
        let peer_ip = peer_ip.ok_or_else(|| bendy::decoding::Error::missing_field("peer_ip"))?;
        let peer_open_port = peer_open_port.ok_or_else(|| bendy::decoding::Error::missing_field("peer_open_port"))?;
        let peer_file_meta_info_report = peer_file_meta_info_report.ok_or_else(|| bendy::decoding::Error::missing_field("peer_file_meta_info_report"))?;
        Ok(PeerInfo::new(peer_id, peer_ip, peer_open_port, peer_file_meta_info_report))
    }
}


#[cfg(test)]
mod test_bencode {
    use bendy::decoding::FromBencode;
    use bendy::encoding::ToBencode;
    use super::{FileMetaInfo, FilePieceInfo, PeerInfo};
    use uuid::Uuid;
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
        let file_meta_info = FileMetaInfo::new(Uuid::new_v4(), sha3_code, OsString::from("luweiba.pdf"), 1<<16, vec![file_piece_info]);
        let encoded = file_meta_info.to_bencode().unwrap();
        let expected_file_meta_info = FileMetaInfo::from_bencode(&encoded).unwrap();
        assert_eq!(expected_file_meta_info, file_meta_info);
    }

    #[test]
    fn test_peer_info_with_bencode() {
        let sha3_code = (0..32).collect::<Vec<u8>>();
        let file_piece_info = FilePieceInfo::new(12, sha3_code.clone());
        let file_meta_info = FileMetaInfo::new(Uuid::new_v4(), sha3_code, OsString::from("luweiba.pdf"), 1<<16, vec![file_piece_info]);
        let peer_info = PeerInfo::new(16, vec![127, 0, 0, 1], 8080, vec![file_meta_info]);
        let encoded = peer_info.to_bencode().unwrap();
        let expected_peer_info = PeerInfo::from_bencode(&encoded).unwrap();
        assert_eq!(expected_peer_info, peer_info);
    }
}