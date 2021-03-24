use std::net::TcpListener;
use std::io;
use std::io::Write;

fn main() -> io::Result<()> {
    let mut connection_info = vec![];
    let listener = TcpListener::bind("127.0.0.1:8003")?;
    for stream in listener.incoming() {
        let mut stream = stream?;
        connection_info.push(stream.peer_addr()?);
        println!("Connection from {}", stream.peer_addr()?);
        stream.write(format!("{:?}", connection_info).as_bytes())?;
    }
    Ok(())
}