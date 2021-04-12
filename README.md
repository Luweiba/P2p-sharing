# p2p-try0
局域网内实现实时文件P2P传输共享
## 实验目的

* 设计一套P2P**文件实时<u>共享</u>协议**，实现如下功能：
  * 每个人的机器上有一个客户端在运行，在机器联网的时候可以在**合理的假设条件下**发现并加入**共享网络**
  * 客户端监听本地的指定文件夹，每个人都可以往文件夹里添加文件，但不能修改和删除文件
  * 协议使得共享网络里的每台机器的指定文件夹的内容保持一致
  * 协议必须使用P2P传输方式
  * 协议需要尽可能的提高所有机器上文件夹内容达到一致的速度，实验报告里请谈谈你们是怎么针对性设计的
* 在协议确定后，组内每个人实现自己的客户端，使用自己习惯的编程语言，也就是说，有几个人，就要交几份客户端源代码，互相抄袭也算抄袭
* Deadline：2021/4/12 23:59

## 实验内容

### 协议设计

分析设计要求可知，实现P2P文件实时共享协议的主要设计难点在于：

1. **分布式一致性**

2. **实时性**

3. **P2P传输**

针对上述难点，本协议做了如下设计：

##### 1. 针对分布式一致性的设计

实现共享的前提是多台客户端主机可以看到彼此的文件信息，此文件信息是全局统一的，即不存在一台主机看不到另外一台主机共享的文件信息这种情况，所以在进行多台客户端主机进行共享时，多台主机都需要维护一个全局统一的信息表，此信息表主要为**（用户信息，文件信息）**二元组的列表。

为了实现多台客户端主机的信息的共享与组织，本协议采用了中心化的设计，即在多台主机中需要有一台主机担任信息表的维护的任务，有这样职能的客户端主机称为**Tracker**，其他没有此职能的主机称为**Peer**。

> **Tips：**
>
> Tracker和Peer只是角色的分化，并不是客户端主机的分化，即一台客户端主机可以既是Tracker又是Peer，但是在一个共享网络中只存在一台客户端主机承担Tracker角色，其他客户端主机可以只有Peer角色。
>
> 具体实现时为了简化工作量，可以将角色分化实现为客户端主机分化。

其中**Tracker**与**Peer**的主要职能如下：

| Tracker                                                      | Peer                             |
| ------------------------------------------------------------ | -------------------------------- |
| 创建共享网络，为Peer分配全局ID，告知其余Peer新加入的Peer信息 | 加入共享网络                     |
| 分发Peer的更新文件信息，维护全局信息表的实时统一             | 实时检测本地更新文件信息         |
| 检测Peer的存活情况，并分发掉线Peer的信息                     | 基于全局信息表向其余Peer请求文件 |

分布式的共识一致性难点在于多台主机通信之间的不可预料的错误，如主机崩溃或网络不畅造成的信息丢失，**此协议假设共享网络中的主机不会崩溃且网络状态良好。**

分布式共识一致性的设计借鉴了部分Raft协议的思想。Raft协议要求每一个term都要选举一个leader来实现全局共识，本协议则固定由一台客户端主机承担leader的职能，如果此主机崩溃了，则共享网络崩溃。而针对可能出现的由于崩溃与网络不畅造成的日志条目不一致问题，本协议假设共享网络中的主机不会崩溃且网络状态良好，从而避免了此部分的复杂设计。本协议并不假设所有客户端主机同时加入共享网络，针对随着时间变化的加入与退出共享网络的主机，本协议借鉴了Raft的Snapshot思想，即后来加入共享网络的主机会直接获得全局信息表的一份Snapshot，从而避免了客户端主机维护LogEntry的复杂实现。

##### 2. 针对实时性的设计

实时性即要求本地的文件信息更新会很快的散布到共享网络中，然后基于更新的文件信息发起新一轮的共享。其中将文件信息散布到共享网络中由**分布式共识一致性模块**承担，实时检测则由一个**本地文件实时检测模块**承担，具体功能为：**定时扫描客户端主机共享的文件目录，如果发现有新加入的文件则解析此文件并将更新的文件信息发送给Tracker，由Tracker将此文件更新信息分发到共享网络中的每一台Peer**。

> Tips：
>
> 对于文件信息的更新，本地文件实时监测模块还需要实现**去重**的功能，因为可能存在同一份文件的多份副本，此副本的文件名在多台主机上可能一致也可能不一致，故在全局信息表的维护时，用于唯一确定文件的并不是文件名，而是文件内容的散列值。但是对于本地检测而言，因为本地文件不允许出现同名的存在，检测时并不需要计算所有文件的散列值然后与上一次扫描的散列值做对比得出文件更新信息，只需要检测新出现的文件名即可，然后针对新出现的文件名，在做进一步的解析，如果此文件仅是本地文件的一份副本，则此文件更新信息不会散布到共享网络中，这样可以大大提升本地文件扫描与解析的速度。

##### 3. 针对P2P传输的设计

P2P传输的优点在于内容下载者同时也是内容上载者，针对P2P传输，首先要解决的问题就是路由问题，即一台Peer可以与另一台Peer通信，因为NAT的广泛部署，现在的上网主机的IP一般都是私有IP地址，而私有IP地址是不可全球路由的，所以**本协议假设此共享分发网络为局域网内的**，即所有Peer包括Tracker都是在统一局域网下进行实时共享的。在全局维护的信息表中，不仅包括了文件信息，也包括Peer信息，其中Peer信息中就包含了Peer的私网IP地址与用于P2P分发的端口号

> Tips：
>
> 对于提高P2P的传输速度，可以采用文件分块的方法，对于每一个文件都进行分块并计算散列值，然后请求的时候可以多个块同时向多个Peer申请，其中请求块的策略可以依照**最稀缺块优先原则**来实现。
>
> 为了简化实现也可以不分块。

### 协议概述

协议中发送的包的格式为：<**Type_ID**<1 byte>><**Payload_length**<4 bytes>><**Payload_content**>

##### 1. 注册流程

1. Tracker启动服务，
2. Peer连接Tracker，并向Tracker发送本地文件信息（这个包的Type_ID为1，是**用户信息注册包**）

3. Tracker收到Peer的用户信息注册包
   1. 给Peer分配Peer ID，并结合Peer的IP信息与开放端口号还有Peer的本地文件信息组合成一条PeerInformation。
   2. Tracker将PeerInformation存入Tracker的全局信息表中。
   3. Tracker向其他的Peer发送此PeerInformation（这个包的Type_ID为2，是**新加入用户信息包**），然后Peer更新全局信息表。
   4. Tracker向请求注册的Peer发送全局信息表的Snapshot（这个包的Type_ID为3，是**全局信息表设置包**），然后Peer设置全局信息表。

结束注册流程后，如果客户端主机共享的文件夹本身就有文件，则开始P2P传输，如果没有文件且客户端主机向该文件夹添加文件，则此部分的文件添加流程如下

##### 2. 文件更新流程

1. Peer扫描本地共享文件夹，发现新添加的文件后解析文件生成FileMetaInformation，此结构体主要包含：**文件名、文件分片大小、文件内容散列值，文件分片信息列表FilePieceInformation**，其中文件分片信息主要包含：**分片索引、分片内容散列值。**
2. Peer将新添加的文件信息更新到本地的全局信息表。
3. Peer将此文件更新信息结合自身的Peer ID发送给Tracker（这个包的Type_ID为4，是**用户新添加文件更新包**）
4. Tracker收到<u>用户新添加文件更新包</u>后更新本地的全局信息表。
5. Tracker将此<u>用户新添加文件更新包</u>发送至共享网络内的其余Peer
6. 其余Peer收到<u>用户新添加文件更新包</u>后更新本地的全局信息表。

##### 3. Tracker存活检测流程

1. Tracker在收到Peer的<u>用户信息注册包</u>后会记录此Peer的IP信息并开启存活检测计时器。
   * 如果在保持联系的检测间隔内收到来自该Peer的心跳维持包（这个包的Type_ID是0，是**心跳维持包**)，则刷新该Peer的存活检测计时器
   * 如果在一定时间间隔内没有收到来自该Peer的心跳维持包
     1. 则在本地全局信息表中删掉此Peer的信息
     2. 给共享网络中的其他Peer发送Peer掉线包（这个包的Type_ID是5，是**Peer掉线更新包**）
2. Peer在收到<u>全局信息表设置包</u>的同时也会得到Tracker的保持联系的检测间隔KeepAliveInterval，然后Peer会定时向Tracker发送心跳维持包来刷新在Tracker维持的存活检测计时器。

##### 4. P2P传输流程

当全局信息表更新时，即收到**全局信息表设置包**、**新加入用户信息包**、**用户新添加文件更新包**时，意味着全局信息表中可能有本地未拥有的文件，此时是可以开启P2P传输请求的。

**P2P传输请求流程**

1. 当收到上述几个包时，Peer的P2P传输模块会根据更新后的全局信息表计算出本地缺少的文件列表

2. 根据文件列表中文件的FileMetaInformation，P2P传输模块会得到文件的分片信息以及拥有文件分片的Peer的IP地址与端口号。

3. P2P传输模块按照`最稀缺块优先原则`对文件分片进行请求包。（这个包的Type_ID为6，是**文件分片请求包**）

4. 当此Peer收到另外一个Peer的P2P传输模块发送的文件分片时，会先将此分片存在缓冲区中，然后在本地的全局信息表中更新此文件分片信息，然后向Tracker发送此文件分片更新信息。（这个包的Type_ID为8，是**文件分片更新包**）。

   > 如果缓冲区中的文件分片数足够了，会将文件分片重新按序组合写入本地，然后删除缓冲区中的文件分片。

5. Tracker收到此<u>文件分片更新包</u>后会修改本地的全局信息表，并将此包发送给共享网络中的其余Peer。

**P2P传输分发流程**

1. 当P2P传输模块收到**文件分片请求包**时，会首先检索缓冲区中是否存在此文件分片，如果没有，则从本地读取对应的文件分片内容，然后发送给请求方。（这个包的Type_ID为7，是**文件分片分发包**）

### 协议实现

> 编程语言：Rust
>
> 使用框架：Tokio异步框架
>
> 外部依赖：
>
> * tokio：提供非阻塞异步I/O
> * sha3：提供SHA3-256散列算法
> * bendy：提供Bencode编码与解码
> * structopt：提供命令行框架，将程序包装为命令行程序
> * log：提供日志系统
> * env_logger：提供环境变量配置日志系统

**首先是报文定义**：

```rust
/// [ Message typeID <1 bytes> ][ Payload length <4 bytes> ][ Payload ]
/// 0 => `Peer`与`Tracker`之间保持联系的心跳包
/// 1 => `Peer`向`Tracker`注册信息的包
/// 2 => `Tracker`向`Peer`发送新加入的PeerInfo信息
/// 3 => `Tracker`向`Peer`发送完整的PeersInfoTable
/// 4 => `Tracker`向`Peer`发送用户更改文件信息用户添加文件信息
/// 5 => `Tracker`向`Peer`发送用户掉线信息
/// 6 => `Peer`向`Peer`发送文件请求的包
/// 7 => `Peer`向`Peer`发送文件块的包
/// 8 => `Tracker`向`Peer`发送用户下载块的信息

/// [ Message typeID <1> ]
#[derive(Debug, Eq, PartialEq)]
pub struct RegisterPayload {
    // 发送本地共享文件信息
    file_meta_info_report: Vec<FileMetaInfo>,
    // 用于共享文件的 端口号
    file_share_port: u16,
    // 用于分发PeersInfo信息的端口号
    peer_info_sync_open_port: u16,
}
/// [ Message typeID <2> ]
/// 2 => `Tracker`向`Peer`发送新加入的PeerInfo信息
pub struct PeerInfoAddPayload {
    peer_info: PeerInfo,
}
/// [ Message typeID <3> ]
#[derive(Debug, Eq, PartialEq)]
pub struct PeersInfoSetPayload {
    peer_id: u32,
    peer_ip: Vec<u8>,
    peers_info_table: PeersInfoTable,
    keep_alive_interval: u64,
    keep_alive_manager_open_port: u16,
}
/// [ Message typeID <4> ]
#[derive(Debug)]
pub struct PeersInfoUpdatePayload {
    peer_id: u32,
    updated_file_meta_info: Vec<FileMetaInfo>,
}
/// [ Message typeID <5> ]
/// 用户掉线信息包
pub struct PeerDroppedPayload {
    peer_id: u32,
}

///  [ Message typeID <6> ]
/// 6 => `Peer`向`Peer`发送文件请求的包
pub struct FilePieceRequestPayload {
    // 文件的sha3_code
    file_sha3_code: Vec<u8>,
    // piece_sha3_code
    file_piece_sha3_code: Vec<u8>,
    // piece_index
    file_piece_index: u32,
    // file_piece_size
    file_piece_size: u32,
}
///  [ Message typeID <8> ]
/// 8 => `Tracker`向`Peer`发送用户下载块的信息
pub struct FilePieceInfoAddPayload {
    peer_id: u32,
    file_piece_info: FilePieceInfo,
    file_meta_info_with_empty_piece: FileMetaInfo,
}
```

上述结构体均实现了Bencode编码的序列化与反序列化

**以下是本地文件分片结构体的定义**：

```rust
/// 定义文件分片信息
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct FilePieceInfo {
    index: u32,
    sha3_code: Vec<u8>,
}
/// 定义文件信息
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct FileMetaInfo {
    sha3_code: Vec<u8>,
    file_name: OsString,
    file_piece_size: u32,
    pieces_info: Vec<FilePieceInfo>,
}
```

**以下是全局信息表存储的条目结构体的定义**：

```rust
/// 定义全局信息表中的Peer信息
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct PeerInfo {
    peer_id: u32,
    peer_ip: Vec<u8>,
    peer_open_port: u16,
    peer_file_meta_info_report: Vec<FileMetaInfo>,
}
```

根据协议设计主要实现分为以下几个模块

| 模块                                         | 功能                                                         |
| -------------------------------------------- | ------------------------------------------------------------ |
| local_file_manager（**本地文件管理模块**）   | 负责本地文件的扫描与更新、本地文件的分发与请求文件的存储     |
| keep_alive_manager（**存活检测模块**）       | （Tracker）负责Peer的存活检测，定时器实现                    |
| peer_to_peer_manager（**P2P传输模块**）      | 负责对等方之间的文件请求与分发                               |
| peer_to_tracker_manager（**P2T通信模块**）   | （Peer）负责与Tracker的通信（发送注册包，本地更新包）        |
| tracker_to_peer_manager（**T2P通信模块**）   | （Tracker）负责与Peer的通信（接收注册包，本地更新包）        |
| peers_info_manager_of_tracker（**Tracker**） | （Tracker）负责全局信息表的维护并根据全局信息表的变动调度各模块 |
| peers_info_manager_of_peer（**Peer**）       | （Peer）负责全局信息表的维护并根据全局信息表的变动调度各模块 |
| p2p_client（**Peer功能整合**）               | 将Peer的功能整合在一起                                       |
| p2p_tracker（**Tracker功能整合**）           | 将Tracker的功能整合在一起                                    |
