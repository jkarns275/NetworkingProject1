use std::cmp::min;
use std::net::*;
use std::sync::mpsc::{ Receiver, channel };
use std::thread;
use std::io::{ Write, Read, BufRead, self };
use std::time::Duration;


use util::pretty_print;
use config::*;

pub fn start_echo_server() -> Result<(), io::Error> {
    let (tcp_send, tcp_recv) = channel();
    let (udp_send, udp_recv) = channel();

    let tcp_handle = thread::spawn(move || { tcp_echo(tcp_recv) });
    let udp_handle = thread::spawn(move || { udp_echo(udp_recv) });

    let stdin = io::stdin();

    let mut s = String::new();

    println!("Successfully started echo server. Press any key to close the echo server.");

    // Wait unti enter is pressed, then send the kill signal to both threads and wait for them
    // to returna
    let mut handle = stdin.lock();
    handle.read_line(&mut s)?;

    // If the tcp thread has closed
    if let Err(e) = tcp_send.send(()) {
        if let Ok(Err(e)) = tcp_handle.join() {
            pretty_print("ERR", "TCP Thread", &format!("TCP thread encountered error '{:?}' while executing", e), false);
        } else {
            pretty_print("ERR", "TCP Thread", &format!("Failed send kill signal to TCP thread, encountered error '{:?}'", e), false);
        }
    } else {
        if let Err(e) = tcp_handle.join() {
            pretty_print("ERR", "TCP Thread", &format!("Failed to join with TCP thread, encountered error {:?}", e), false);
        } else {
            pretty_print("LOG", "TCP Thread", "Successfully closed TCP thread.", false);
        }
    }

    // If the udp thread has closed
    if let Err(e) = udp_send.send(()) {
        if let Ok(Err(e)) = udp_handle.join() {
            pretty_print("ERR", "UDP Thread", &format!("UDP thread encountered error '{:?}' while executing", e), false);
        } else {
            pretty_print("ERR", "UDP Thread", &format!("Failed send kill signal to UDP thread, encountered error '{:?}'", e), false);
        }
    } else {
        if let Err(e) = udp_handle.join() {
            pretty_print("ERR", "UDP Thread", &format!("Failed to join with UDP thread, encountered error {:?}", e), false);
        } else {
            pretty_print("LOG", "UDP Thread", "Successfully closed UDP thread.", false);
        }
    }

    Ok(())

}

#[allow(deprecated)]
pub fn tcp_echo(exit_recv: Receiver<()>) -> Result<(), io::Error> {
    let tcp = match TcpListener::bind(ECHO_SERVER_TCP_IP) {
        Ok(x) => x,
        Err(e) => {
            pretty_print("ERR", "Echo Server",
                         &format!("Failed to create TcpListener with ip {}, encountered error '{}'", ECHO_SERVER_TCP_IP, e), false);
            return Err(e)
        }
    };
    tcp.set_nonblocking(true)?;

    // 64 MB buffer
    let mut buffer = vec![0u8; 1024 * 1024 * 64];

    loop {
        if let Ok((mut tcp_stream, socket_addr)) = tcp.accept() {
            tcp_stream.set_nonblocking(false)?;
            tcp_stream.set_read_timeout(None)?;
            loop {
                if let Ok(bytes_read) = tcp_stream.read(&mut buffer) {
                    if bytes_read == 0 {
                        pretty_print("LOG", "Echo Server", &format!("Closing TcpStream with address {:?}", socket_addr), false);
                        break
                    }
                    match tcp_stream.write(&buffer[0..bytes_read]) {
                        Ok(_) => pretty_print("LOG", "Echo Server", &format!("Successfully echoed {} bytes: {:?}", bytes_read, &buffer[0..min(bytes_read, 4)]), false),
                        _ => pretty_print("ERR", "Echo Server", &format!("Failed to echo {} bytes back", bytes_read), false),
                    }
                } else {
                    pretty_print("ERR", "Echo Server", &format!("Failed to read any bytes from socket with address {:?}", socket_addr), false);
                }
                // To reduce CPU usage
                thread::sleep_ms(1);
            }
        }
        // So the program doesn destroy the cpu / battery of my laptop
        thread::sleep_ms(10);
        if let Ok(_) = exit_recv.try_recv() {
            return Ok(())
        }
    }
}

#[allow(deprecated)]
pub fn udp_echo(exit_recv: Receiver<()>) -> Result<(), io::Error> {
    let udp = match UdpSocket::bind(ECHO_SERVER_UDP_IP) {
        Ok(x) => x,
        Err(e) => {
            pretty_print("ERR", "Echo Server",
                         &format!("Failed to create UdpSocket with ip {}, encountered error '{}'", ECHO_SERVER_UDP_IP, e), false);
            return Err(e)
        }
    };

    let _ = udp.set_read_timeout(Some(Duration::from_secs(1)));

    // We want to send data back to the actual udp_dst
    let udp_dst = UDP_IP.parse::<SocketAddr>().unwrap();

    // 64 MB will be way more than enough
    let mut buffer = vec![0u8; 1024 * 1024 * 64];

    // Keep trying to receive data until we get the kill signal, then return
    loop {

        if let Ok((bytes_read, _socket_addr)) = udp.recv_from(&mut buffer) {
            match udp.send_to(&buffer[0..bytes_read], &udp_dst) {
                Ok(_)   => pretty_print("LOG", "Echo Server", &format!("Successfully echoed {} bytes: {:?}", bytes_read, &buffer[0..min(bytes_read, 4)]), false),
                _       => pretty_print("ERR", "Echo Server", &format!("Failed to echo {} bytes back", bytes_read), false),
            }
        }

        if let Ok(_) = exit_recv.try_recv() {
            return Ok(())
        }

        thread::sleep_ms(1);
    }

}