use p2p_try0::p2p_tracker::P2PTracker;
use std::error::Error;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "P2P Tracker")]
struct TrackerOpt {
    #[structopt(short = "d")]
    local_base_directory: PathBuf,
    #[structopt(long, short)]
    tracker_ip: String,
    #[structopt(long, short = "p", default_value = "8013")]
    tracker_open_port: u16,
    #[structopt(long, default_value = "10000")]
    keep_alive_interval: u64,
    #[structopt(long, default_value = "8091")]
    p2p_open_port: u16,
    #[structopt(long, default_value = "8081")]
    sync_open_port: u16,
    #[structopt(long, default_value = "10000")]
    scanning_interval: u64,
    #[structopt(long, default_value = "8004")]
    keep_alive_open_port: u16,
    #[structopt(long, default_value = "65536")]
    piece_size: u32,
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let opt: TrackerOpt = TrackerOpt::from_args();
    // tracker_ip: Ipv4Addr,
    //         tracker_port: u16,
    //         interval: u32,
    //         local_bind_ip: Ipv4Addr,
    //         open_port: u16,
    //         local_base_directory: PathBuf,
    //         piece_size: u32,
    let tracker_ip = Ipv4Addr::from_str(opt.tracker_ip.as_str())?;
    let mut p2p_tracker = P2PTracker::new(
        tracker_ip,
        opt.tracker_open_port,
        10000,
        opt.p2p_open_port,
        opt.local_base_directory,
        opt.piece_size,
        opt.scanning_interval,
        opt.keep_alive_interval,
        opt.keep_alive_open_port,
    );
    p2p_tracker.start_tracking().await?;
    loop {}
    Ok(())
}
