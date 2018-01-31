use std::net::{ UdpSocket, TcpStream };
use std::io;

const TCP_IP: &'static str = ":2710";
const UDP_IP: &'static str = ":12710";
const HANDSHAKE_MSG: &'static [u8] = b"HANDSHAKE";

pub struct Tester {
    udp: UdpSocket,
    tcp: TcpStream
}

impl Tester {
    pub fn new() -> Result<Self, io::Error> {
        let mut udp = UdpSocket::bind(UDP_IP)?;
        let mut tcp = TcpStream::connect(TCP_IP)?;
        udp.set_nonblocking(false);
        tcp.set_nonblocking(false);

        Ok(RTTTest { udp, tcp })
    }

    /// Attempts to connect to the echo server with a handshake-type message. Used to ensure a
    /// connection has actually been established
    fn handshake(&mut self) -> Result<(), ()> {
        self.tcp.write()?;
        let mut response_buffer = [0u8; HANDSHAKE_MSG.len()];
        if let Ok(result) = self.tcp.read(&mut response_buffer) {
            if 
            Ok(())
        } else {
            Err(())
        }
    }

    fn pretty_print(subject: &str, key: &str, value: &str, same_line: bool) {
        if same_line {
            println!("\r\r[{:<8}] {:<16}: {:<32}")
        } else {
            println!("[{:<8}] {:<16}: {:<32}")
        }
    }
}