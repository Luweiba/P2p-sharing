use p2p_try0::p2p_client::P2PClient;
use std::error::Error;
use std::io;
use std::io::Read;
use std::net::{Ipv4Addr, TcpStream};
use std::path::PathBuf;
use tokio::runtime::Builder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let local_bind_ip = Ipv4Addr::from([127, 0, 0, 1]);
    let open_port = 8090;
    let local_base_directory =
        PathBuf::from("C:\\Users\\32050\\Desktop\\计算机网络\\dynamic secrets");
    let mut client = P2PClient::new(local_bind_ip, open_port, local_base_directory, 1 << 16);
    client
        .connect_to_tracker(Ipv4Addr::from([127, 0, 0, 1]), 8003)
        .await?;
    Ok(())
}
