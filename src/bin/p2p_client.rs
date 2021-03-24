use std::net::TcpStream;
use std::io;
use std::io::Read;

fn main() -> io::Result<()> {
    let mut connection = TcpStream::connect("127.0.0.1:8003")?;
    let mut buf = [0u8; 512];
    let nbytes = connection.read(&mut buf)?;
    println!("Info: {}", String::from_utf8_lossy(&buf[..nbytes]));
    Ok(())
}