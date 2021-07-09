use p2p_try0::p2p_client::P2PClient;
use std::error::Error;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "P2P Peer")]
struct ClientOpt {
    #[structopt(long, short)]
    tracker_ip: String,
    #[structopt(long, short = "p", default_value = "8013")]
    tracker_open_port: u16,
    #[structopt(short = "d")]
    local_base_directory: PathBuf,
    #[structopt(long, default_value = "0.0.0.0")]
    local_bind_ip: String,
    #[structopt(long, default_value = "8090")]
    p2p_open_port: u16,
    #[structopt(long, default_value = "8080")]
    sync_open_port: u16,
    #[structopt(long, default_value = "10000")]
    scanning_interval: u64,
    #[structopt(long, default_value = "8005")]
    local_keep_alive_open_port: u16,
    #[structopt(long, default_value = "65536")]
    piece_size: u32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    /*
    let local_bind_ip = Ipv4Addr::from([127, 0, 0, 1]);
    let open_port = 8090;
    let peer_info_sync_open_port = 8080;
    let local_base_directory = PathBuf::from("C:\\Users\\32050\\Desktop\\test2");
    let scanning_interval = 10000;
    let local_keep_alive_open_port = 8005;
    */
    env_logger::init();
    let opt: ClientOpt = ClientOpt::from_args();
    let local_bind_ip = Ipv4Addr::from_str(opt.local_bind_ip.as_str())?;
    let tracker_ip = Ipv4Addr::from_str(opt.tracker_ip.as_str())?;
    let mut client = P2PClient::new(
        local_bind_ip,
        opt.p2p_open_port,
        opt.local_base_directory,
        opt.piece_size,
    );
    client
        .connect_to_tracker(
            tracker_ip,
            opt.tracker_open_port,
            opt.sync_open_port,
            opt.scanning_interval,
            opt.local_keep_alive_open_port,
        )
        .await?;
    loop {}
    Ok(())
}
