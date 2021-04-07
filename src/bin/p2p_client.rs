use p2p_try0::p2p_client::P2PClient;
use std::error::Error;
use std::net::Ipv4Addr;
use std::path::PathBuf;







#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let local_bind_ip = Ipv4Addr::from([127, 0, 0, 1]);
    let open_port = 8090;
    let peer_info_sync_open_port = 8080;
    let local_base_directory = PathBuf::from("C:\\Users\\32050\\Desktop\\test2");
    let scanning_interval = 10000;
    let local_keep_alive_open_port = 8005;
    let mut client = P2PClient::new(local_bind_ip, open_port, local_base_directory, 1 << 16);
    client
        .connect_to_tracker(
            Ipv4Addr::from([127, 0, 0, 1]),
            8003,
            peer_info_sync_open_port,
            scanning_interval,
            local_keep_alive_open_port,
        )
        .await?;
    loop {}
    Ok(())
}
