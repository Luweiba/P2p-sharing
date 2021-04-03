use crate::thread_communication_message::KAToPIMessage;
use std::collections::HashMap;
use std::error::Error;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{mpsc, Mutex, oneshot};

#[derive(Debug)]
struct KeepAliveTimer {
    keep_alive_interval: u64,
    expected_count: Arc<Mutex<u32>>,
    inner_count: Arc<Mutex<u32>>,
    dropped_info_sender: Sender<u32>,
    peer_id: u32,
}

impl KeepAliveTimer {
    pub async fn new(interval: u64, peer_id: u32, dropped_info_sender: Sender<u32>) -> Self {
        let mut inner_count = Arc::new(Mutex::new(0u32));
        let expected_count = Arc::new(Mutex::new(0u32));
        Self {
            keep_alive_interval: interval,
            expected_count,
            inner_count,
            dropped_info_sender,
            peer_id,
        }
    }

    pub async fn flush_timing(&mut self) {
        // 计时 + 1
        let expected_count_clone = self.expected_count.clone();
        {
            let mut expected_count_lock = expected_count_clone.lock().await;
            *expected_count_lock += 1;
        }
        // 释放expected_count_lock锁
        let (sender, mut receiver) = oneshot::channel::<()>();
        let interval = self.keep_alive_interval.clone();
        println!("Interval {}", interval);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(interval)).await;
            sender.send(());
        });
        let dropped_info_sender_clone = self.dropped_info_sender.clone();
        let expected_count_clone = self.expected_count.clone();
        let inner_count_clone = self.inner_count.clone();
        let peer_id_clone = self.peer_id;
        tokio::spawn(async move {
            receiver.await;
            // 获取inner_count_lock
            let inner_count_snapshot;
            {
                let mut inner_count_lock = inner_count_clone.lock().await;
                *inner_count_lock += 1;
                inner_count_snapshot = *inner_count_lock;
            }
            {
                let expected_count_lock = expected_count_clone.lock().await;
                if inner_count_snapshot == *expected_count_lock {
                    dropped_info_sender_clone.send(peer_id_clone).await;
                }
            }
            // 释放expected_count_lock
        });
    }

}
#[derive(Debug)]
pub struct KeepAliveIPTable {
    ip_to_peer_id_and_timer_map: HashMap<Ipv4Addr, (u32, KeepAliveTimer)>,
}

impl KeepAliveIPTable {
    pub fn new() -> Self {
        Self {
            ip_to_peer_id_and_timer_map: HashMap::new(),
        }
    }

    pub async fn flush_timing(&mut self, peer_ip: Ipv4Addr) {
        if self.ip_to_peer_id_and_timer_map.contains_key(&peer_ip) {
            let (_, timer) = self.ip_to_peer_id_and_timer_map.get_mut(&peer_ip).unwrap();
            timer.flush_timing().await;
        }
    }

    pub async fn add_one_peer(&mut self, peer_id: u32, peer_ip: Ipv4Addr, keep_alive_interval: u64, dropped_info_sender: Sender<u32>) {
        let mut timer = KeepAliveTimer::new(keep_alive_interval, peer_id, dropped_info_sender).await;
        timer.flush_timing().await;
        self.ip_to_peer_id_and_timer_map.insert(peer_ip, (peer_id, timer));
        println!("KeepAliveManager 添加 Peer {}", peer_id);
    }

    pub fn remove_one_peer(&mut self, peer_id: u32) {
        let mut ip_addr= Ipv4Addr::new(0, 0, 0, 0);
        for (key, value) in self.ip_to_peer_id_and_timer_map.iter() {
            if value.0 == peer_id {
                ip_addr = key.clone();
                break;
            }
        }
        if ip_addr != Ipv4Addr::new(0,0,0,0) {
            self.ip_to_peer_id_and_timer_map.remove(&ip_addr);
            println!("KeepAliveTable 已删除 peer {}", peer_id);
        }
    }
}

#[derive(Debug)]
pub struct KeepAliveManager {
    tracker_ip: Ipv4Addr,
    keep_alive_interval: u64,
    keep_alive_open_port: u16,
    keep_alive_ip_table: Arc<Mutex<KeepAliveIPTable>>,
}

impl KeepAliveManager {
    pub fn new(tracker_ip: Ipv4Addr, keep_alive_interval: u64, keep_alive_open_port: u16) -> Self {
        Self {
            tracker_ip,
            keep_alive_interval,
            keep_alive_open_port,
            keep_alive_ip_table: Arc::new(Mutex::new(KeepAliveIPTable::new())),
        }
    }

    pub async fn start_monitoring(
        &mut self,
        pi_sender: Sender<KAToPIMessage>,
        mut pi_receiver: Receiver<KAToPIMessage>,
    ) -> Result<(), Box<dyn Error>> {
        let mut keep_alive_socket = UdpSocket::bind(SocketAddr::from(SocketAddrV4::new(
            self.tracker_ip,
            self.keep_alive_open_port,
        )))
        .await?;
        println!(
            "Tracker Keep Alive Manager bind ip: {:?}",
            SocketAddr::from(SocketAddrV4::new(
                self.tracker_ip,
                self.keep_alive_open_port
            ))
        );

        // 处理Peer掉线信息
        let (dropped_peer_id_sender, mut dropper_peer_id_receiver) = mpsc::channel::<u32>(10);
        let keep_alive_ip_table_clone = self.keep_alive_ip_table.clone();
        tokio::spawn(async move {
            println!("开始监听心跳包");
            loop {
                if let Some(dropped_peer_id) = dropper_peer_id_receiver.recv().await {
                    // TODO 发送掉线信息给PI manager
                    {
                        let mut keep_alive_ip_table_lock = keep_alive_ip_table_clone.lock().await;
                        keep_alive_ip_table_lock.remove_one_peer(dropped_peer_id);
                    }
                    let message = KAToPIMessage::new_peer_dropped_messgae(dropped_peer_id);
                    println!("Peer {} 掉线了", dropped_peer_id);
                    pi_sender.send(message).await;
                }
            }
        });
        // 处理新加入的IP地址
        let pi_keep_alive_ip_table = self.keep_alive_ip_table.clone();
        let pi_keep_alive_interval = self.keep_alive_interval;
        tokio::spawn(async move {
            loop {
                if let Some(message) = pi_receiver.recv().await {
                    match message {
                        KAToPIMessage::PeerOnlineMessage { peer_id, peer_ip} => {
                            // 处理Peer上线
                            // 获取全局锁
                            {
                                let mut keep_alive_ip_table_lock = pi_keep_alive_ip_table.lock().await;
                                keep_alive_ip_table_lock.add_one_peer(peer_id, peer_ip, pi_keep_alive_interval, dropped_peer_id_sender.clone()).await;
                            }
                        },
                        _ => {}
                    }
                }
            }
        });
        let mut buf = vec![0u8; 256];
        let keep_alive_ip_table_clone = self.keep_alive_ip_table.clone();
        loop {
            if let Ok((nbytes, peer_addr)) = keep_alive_socket.recv_from(&mut buf[..]).await {
                println!("get a packet from {:?}, packet size {}", peer_addr, nbytes);
                // TODO 处理Peer发来的心跳包
                if nbytes < 5 {
                    continue;
                }
                if peer_addr.is_ipv4() {
                    if let SocketAddr::V4(peer_addr_v4) = peer_addr {
                        let peer_ip = peer_addr_v4.ip().clone();
                        {
                            let mut keep_alive_ip_table_lock =
                                keep_alive_ip_table_clone.lock().await;
                            keep_alive_ip_table_lock.flush_timing(peer_ip).await;
                        }
                        // 释放keep_alive_ip_table_lock
                    }
                }
            }
        }
        Ok(())
    }
}
