use p2p_try0::p2p_tracker::P2PTracker;
use std::error::Error;
use std::net::Ipv4Addr;
use std::path::PathBuf;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let tracker_ip = Ipv4Addr::new(127, 0, 0, 1);
    let tracker_port = 8003u16;
    let interval = 100u32;
    let open_port = 8088u16;
    let local_base_directory = PathBuf::from("C:\\Users\\32050\\Desktop\\test1");
    let piece_size = 1 << 16;
    let scanning_interval = 10000;
    let keep_alive_open_port = 8004;
    let keep_alive_interval = 10000;
    // tracker_ip: Ipv4Addr,
    //         tracker_port: u16,
    //         interval: u32,
    //         local_bind_ip: Ipv4Addr,
    //         open_port: u16,
    //         local_base_directory: PathBuf,
    //         piece_size: u32,
    let mut p2p_tracker = P2PTracker::new(
        tracker_ip,
        tracker_port,
        interval,
        open_port,
        local_base_directory,
        piece_size,
        scanning_interval,
        keep_alive_interval,
        keep_alive_open_port,
    );
    p2p_tracker.start_tracking().await?;
    loop {}
    Ok(())
}
