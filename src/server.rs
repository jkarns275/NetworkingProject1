use std::net::{ UdpSocket, TcpStream, TcpListener, SocketAddr, ToSocketAddrs };
use std::io;
use std::io::{ BufRead, Read, Write };
use std::sync::mpsc::{ Sender, Receiver, channel };
use std::thread;
use std::time::{ Duration, Instant };
use std::cmp::min;
use std::hash::{ Hash, Hasher };
use std::collections::hash_map::DefaultHasher;

use test::*;
use util::pretty_print;

// Elementary os localhost ip = 129.3.147.51
const TCP_IP: &'static str = "129.3.120.50:2710";
const UDP_IP: &'static str = "129.3.120.50:12710";
const HANDSHAKE_MSG: &'static [u8] = b"HANDSHAKE";
const ECHO_SERVER_UDP_IP: &'static str = "129.3.20.92:22710";
const ECHO_SERVER_TCP_IP: &'static str = "129.3.20.92:32710";

const MAX_ATTEMPTS: u32 = 16;

pub struct Server {
    udp: UdpSocket,
    udp_dst: SocketAddr,
    tcp: TcpStream,
}

impl Server {
    pub fn new() -> Result<Self, io::Error> {
        let udp = UdpSocket::bind(UDP_IP)?;
        let udp_dst = Server::create_address(ECHO_SERVER_UDP_IP).unwrap();
        let tcp = TcpStream::connect(ECHO_SERVER_TCP_IP)?;
        udp.set_nonblocking(false)?;
        tcp.set_nonblocking(false)?;

        Ok(Server { udp, udp_dst, tcp, })
    }

    pub fn create_address(s: &str) -> Result<SocketAddr, ()> {
        let mut iter = s.to_socket_addrs();
        if iter.is_err() {
            Err(())
        } else if let Some(x) = iter.unwrap().next() {
            Ok(x)
        } else {
            Err(())
        }
    }

    pub fn tcp_echo(exit_recv: Receiver<()>) -> Result<(), io::Error> {
        let mut tcp = match TcpListener::bind(ECHO_SERVER_TCP_IP) {
            Ok(x) => x,
            Err(e) => {
                pretty_print("ERR", "Echo Server",
                             &format!("Failed to create TcpListener with ip {}, encountered error '{}'", ECHO_SERVER_TCP_IP, e), false);
                return Err(e)
            }
        };
        tcp.set_nonblocking(true);

        // 64 MB buffer
        let mut buffer = vec![0u8; 1024 * 1024 * 64];

        loop {
            if let Ok((mut tcp_stream, socket_addr)) = tcp.accept() {
                tcp_stream.set_nonblocking(false);
                tcp_stream.set_read_timeout(None);
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

    pub fn udp_echo(exit_recv: Receiver<()>) -> Result<(), io::Error> {
        let mut udp = match UdpSocket::bind(ECHO_SERVER_UDP_IP) {
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

    pub fn echo() -> Result<(), io::Error> {
        let (tcp_send, tcp_recv) = channel();
        let (udp_send, udp_recv) = channel();

        let tcp_handle = thread::spawn(move || { Server::tcp_echo(tcp_recv) });
        let udp_handle = thread::spawn(move || { Server::udp_echo(udp_recv) });

        let stdin = io::stdin();

        let mut s = String::new();

        println!("Successfully started echo server. Press any key to close the echo server.");

        // Wait unti enter is pressed, then send the kill signal to both threads and wait for them
        // to return.
        let mut handle = stdin.lock();
        handle.read_line(&mut s);

        let tcp_send_res = tcp_send.send(());
        let udp_send_res = udp_send.send(());

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

    /// Attempts to connect to the echo server with a handshake-type message. Used to ensure a
    /// connection has actually been established
    fn handshake(&mut self) -> Result<(), io::Error> {
        pretty_print("LOG", "Handshake", "Beginning handshake.", false);
        self.tcp.write(HANDSHAKE_MSG)?;
        let mut response_buffer = vec![0u8; HANDSHAKE_MSG.len()];
        let bytes_written = self.tcp.read(&mut response_buffer)?;

        if bytes_written == response_buffer.len() && &response_buffer[..] == HANDSHAKE_MSG {
            pretty_print("LOG", "Handshake", "Successfully completed handshake.", false);
            Ok(())
        } else {
            pretty_print("ERR", "Handshake", "Failed to complete handshake with echo server.", false);
            Err(io::Error::new(io::ErrorKind::Other, "Failed to complete handshake with echo server."))
        }
    }

    pub fn run_tests(&mut self, tests: Vec<Test>) -> Result<Vec<TestResult>, io::Error> {
        self.handshake()?;
        Ok(tests.into_iter().map(|x| self.run_test(x)).collect())
    }

    pub fn run_test(&mut self, test: Test) -> TestResult {
        match test {
            Test::UdpTest(spec) => self.run_udp_test(spec),
            Test::TcpTest(spec) => self.run_tcp_test(spec)
        }
    }

    fn tcp_connect(&mut self) -> Result<(), io::Error> {
        let tcp = TcpStream::connect(ECHO_SERVER_TCP_IP)?;
        Ok(())
    }

    fn udp_read_exact(&mut self, mut buf: &mut [u8]) -> Result<(), Option<io::Error>> {
        while buf.len() != 0 {
            let (bytes_written, src_address) = self.udp.recv_from(buf)?;
            if src_address != self.udp_dst {
                return Err(None)
            }
            let tmp = buf;
            buf = &mut tmp[bytes_written..]
        }
        Ok(())
    }

    fn run_udp_test(&mut self, test_spec: TestSpec) -> TestResult {

        let mut s = DefaultHasher::new();
        test_spec.hash(&mut s);
        let test_hash = s.finish();
        let test_string = format!("Test #{}", test_hash);
        let mut durations: Vec<Option<Duration>> = vec![];
        let mut message = vec![0u8; test_spec.message_len];

        self.udp.set_read_timeout(Some(Duration::from_secs(5)));


        pretty_print("LOG", &test_string, &format!("Beginning UDP test with test spec {:?}\n", test_spec), false);

        for i in 0..test_spec.num_messages {
            if message.len() < 4 {
                for j in 0..message.len() {
                    message[j] = ((i >> (j * 8)) & 0xFF) as u8;
                }
            } else {
                for p in 0..message.len() / 4 {
                    message[p * 4 + 0] = (i & 0xFF) as u8;
                    message[p * 4 + 1] = ((i >> 8) & 0xFF) as u8;
                    message[p * 4 + 2] = ((i >> 16) & 0xFF) as u8;
                    message[p * 4 + 3] = ((i >> 24) & 0xFF) as u8;
                }
            }

            let now = Instant::now();

            // Try to send the data. If this fails, continue to the next message after pushing
            // None for the duration of this message
            match self.udp.send_to(&message, self.udp_dst) {
                Ok(bytes_sent) => pretty_print("LOG", &test_string,
                                       &format!("Sent message #{}", i), true),
                Err(e) => {
                    pretty_print("LOG", &test_string,
                                 &format!("Failed to send message #{}, encountered error {:?}", i, e),
                                 false);
                    // Failed to send the packet, so there is no duration for this message
                    durations.push(None);
                    continue
                }
            };

            // Try to receive data from the echo serve MAX_ATTEMPTS times. If it fails to receive
            // data from the correct address, or simply fails for some other reason too many times,
            // None is pushed to durations and the next message is tried.
            for attempt_n in 0..MAX_ATTEMPTS + 1 {
                // We've already tried it the maximum number of times
                if attempt_n == MAX_ATTEMPTS {
                    pretty_print("LOG", &test_string, &format!("Attempted to receive data the maximum number of times for message {}.", i), false);
                    durations.push(Some(now.elapsed()));
                    break
                }
                // Actually receive data, ensure the source is the proper address
                match self.udp.recv_from(&mut message) {
                    Err(e) => {
                        pretty_print("ERR", &test_string, &format!("Encountered error {:?} while trying to receive data. Trying again.", e), false);
                        continue
                    }
                    Ok((bytes_read, src_addr)) => {
                        // Wrong source
                        if src_addr != self.udp_dst {
                            pretty_print("LOG", &test_string,
                                         &format!("Received {} bytes from address {}, expected source address to be {}. Trying again.", bytes_read, src_addr, self.udp_dst),
                                         false);
                            continue

                        } else {
                            // Check if its the same data we sent (all bytes set to i)

                            if (0..message.len())
                                .map(|x| message[x] == ((i >> (8 * (x & 3))) & 0xFF) as u8)
                                .fold(true, |x, y| x && y) {
                                durations.push(Some(now.elapsed()));
                                break
                            } else {
                                // On the case that message.len() < 4

                                continue
                            }
                        }
                    },
                }
            }

        }

        let mut dropped_messages: Vec<u32> = vec![];
        let mut total: Duration = Duration::new(0, 0);
        for i in 0usize..durations.len() {
            if let Some(message_duration) = durations[i] {
                total += message_duration;
            } else {
                    dropped_messages.push(i as u32);
            }
        }

        Ok(TestData {
            dropped_messages,
            test_spec,
            individual_durations: durations,
            total_duration: total,
        })
    }

    fn run_tcp_test(&mut self, test_spec: TestSpec) -> TestResult {

        let mut s = DefaultHasher::new();
        test_spec.hash(&mut s);
        let test_hash = s.finish();
        let test_string = format!("Test #{}", test_hash);
        let mut durations: Vec<Option<Duration>> = vec![];
        let mut message = vec![0u8; test_spec.message_len];

        self.udp.set_read_timeout(Some(Duration::from_secs(5)));


        pretty_print("LOG", &test_string, &format!("Beginning TCP test with test spec {:?}\n", test_spec), false);

        for i in 0..test_spec.num_messages {
            if message.len() < 4 {
                for j in 0..message.len() {
                    message[j] = ((i >> (j * 8)) & 0xFF) as u8;
                }
            } else {
                for p in 0..message.len() / 4 {
                    message[p * 4 + 0] = (i & 0xFF) as u8;
                    message[p * 4 + 1] = ((i >> 8) & 0xFF) as u8;
                    message[p * 4 + 2] = ((i >> 16) & 0xFF) as u8;
                    message[p * 4 + 3] = ((i >> 24) & 0xFF) as u8;
                }
            }

            let now = Instant::now();

            // Try to send the data. If this fails, continue to the next message after pushing
            // None for the duration of this message
            match self.tcp.write_all(&message) {
                Ok(bytes_sent) => pretty_print("LOG", &test_string,
                                               &format!("Sent message #{}", i), true),
                Err(e) => {
                    pretty_print("LOG", &test_string,
                                 &format!("Failed to send message #{}, encountered error {:?}", i, e),
                                 false);
                    // Failed to send the packet, so there is no duration for this message
                    durations.push(None);
                    continue
                }
            };

            // Try to receive data from the echo serve MAX_ATTEMPTS times. If it fails to receive
            // data from the correct address, or simply fails for some other reason too many times,
            // None is pushed to durations and the next message is tried.
            for attempt_n in 0..MAX_ATTEMPTS + 1 {
                // We've already tried it the maximum number of times
                if attempt_n == MAX_ATTEMPTS {
                    pretty_print("LOG", &test_string, &format!("Attempted to receive data the maximum number of times for message {}.", i), false);
                    durations.push(Some(now.elapsed()));
                    break
                }
                // Actually receive data, ensure the source is the proper address
                match self.tcp.read_exact(&mut message) {
                    Ok(bytes_read) => {
                        // Check if its the same data we sent (all bytes set to i)
                        if (0..message.len())
                                .map(|x| message[x] == ((i >> (8 * (x & 3))) & 0xFF) as u8)
                                .fold(true, |x, y| x && y) {
                            durations.push(Some(now.elapsed()));
                            break
                        // It was something else, so lets just ignore it and try again
                        } else {
                            continue
                        }
                    },
                    Err(e) => {
                        pretty_print("ERR", &test_string, &format!("Encountered error {:?} while trying to receive data. Trying again.", e), false);
                        continue
                    }
                }
            }
        }

        let mut dropped_messages: Vec<u32> = vec![];
        let mut total: Duration = Duration::new(0, 0);
        for i in 0usize..durations.len() {
            if let Some(message_duration) = durations[i] {
                total += message_duration;
            } else {
                dropped_messages.push(i as u32);
            }
        }

        Ok(TestData {
            dropped_messages,
            test_spec,
            individual_durations: durations,
            total_duration: total,
        })
    }
}
