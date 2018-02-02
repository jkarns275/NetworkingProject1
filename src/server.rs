use std::net::*;
use std::io::{ Read, Write, self };
use std::time::{ Duration, Instant };
use std::hash::{ Hash, Hasher };
use std::collections::hash_map::DefaultHasher;
use std::mem;

use test::*;
use config::*;
use util::*;

#[allow(non_snake_case)]
fn TIMEOUT_DURATION() -> Duration { Duration::new(10, 0) }

pub struct Server {
    udp: UdpSocket,
    udp_dst: SocketAddr,
    tcp: TcpStream,
}

impl Server {
    pub fn new() -> Result<Self, io::Error> {
        let udp = UdpSocket::bind(UDP_IP)?;
        let udp_dst = create_address(ECHO_SERVER_UDP_IP).unwrap();
        let tcp = TcpStream::connect(ECHO_SERVER_TCP_IP)?;
        udp.set_nonblocking(false)?;
        tcp.set_nonblocking(false)?;

        udp.set_read_timeout(Some(TIMEOUT_DURATION()))?;
        tcp.set_read_timeout(Some(TIMEOUT_DURATION()))?;

        Ok(Server { udp, udp_dst, tcp, })
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

    /// Attempts to read enough bytes to fill [buf], from the proper address (the address of the
    /// echo server). If more than [TIMEOUT_DURATION] seconds pass, this method fails and returns
    /// Err(None).
    fn udp_read_exact(&mut self, mut buf: &mut [u8]) -> Result<(), Option<io::Error>> {
        let initial_len = buf.len();
        let start_time = Instant::now();
        loop {
            let (bytes_written, src_address) = self.udp.recv_from(buf)?;
            if src_address != self.udp_dst {
                return Err(None)
            }

            if bytes_written == buf.len() {
                break
            }

            let tmp = buf;
            buf = &mut tmp[bytes_written..];

            if start_time.elapsed() > TIMEOUT_DURATION() {
                pretty_print("ERR", "UDP Timeout", &format!("Timed out trying to receive {} bytes.", initial_len), false);
                return Err(None)
            }
        }
        Ok(())
    }

    /// Attempts to read enough to fill [buf]. If more than [TIMEOUT_DURATION] seconds pass, this
    /// method fails and returns Err(None)
    fn tcp_read_exact(&mut self, mut buf: &mut [u8]) -> Result<(), Option<io::Error>> {
        let initial_len = buf.len();
        let start_time = Instant::now();
        loop {
            let bytes_written = self.tcp.read(buf)?;

            if bytes_written == buf.len() {
                break
            }

            let tmp = buf;
            buf = &mut tmp[bytes_written..];

            if start_time.elapsed() > TIMEOUT_DURATION() {
                pretty_print("ERR", "TCP Timeout",
                             &format!("Timed out trying to receive {} bytes.", initial_len),
                             false);
                return Err(None)
            }
        }
        Ok(())
    }

    /// Sends a udp message to the echo server, and waits for it to be echoed back.
    fn udp_message(&mut self, message: &mut [u8], test_string: &str, message_number: u32) -> Option<Duration> {
        // Copy the current message number (nth message), which is a u32, into the message.
        let message_bytes = unsafe { mem::transmute::<u32, [u8; 4]>(message_number) };
        for p in 0..message.len() {
            message[p] = message_bytes[p & 3];
        }

        let now = Instant::now();

        // Try to send the data. If this fails, continue to the next message after pushing
        // None for the duration of this message
        match self.udp.send_to(message, self.udp_dst) {
            Ok(_bytes_sent) => {
                pretty_print("LOG",
                             &test_string,
                             &format!("Sent message #{}", message_number),
                             true)
            },
            Err(e) => {
                pretty_print("LOG",
                             &test_string,
                             &format!("Failed to send message #{}, encountered error {:?}", message_number, e),
                             false);
                // Failed to send the packet, so there is no duration for this message
                return None;
            }
        };

        // Actually receive data, ensure the source is the proper address
        match self.udp_read_exact(message) {
            Err(Some(e)) => {
                pretty_print("ERR",
                             &test_string,
                             &format!("Encountered error {:?} while trying to receive data. Trying again.", e),
                             false);
                None
            },
            Err(None) => {
                pretty_print("ERR",
                             &test_string,
                             "Timed out while trying to receive data.",
                             false);
                None
            },
            Ok(()) => {
                // Check if its the same data we sent (all bytes set to i)

                if (0..message.len())
                    .map(|x| message[x] == message_bytes[x & 3])
                    .fold(true, |x, y| x && y) {
                    Some(now.elapsed())
                } else {
                    None
                }
            },
        }
    }
    fn run_udp_test(&mut self, test_spec: TestSpec) -> TestResult {

        let mut s = DefaultHasher::new();
        test_spec.hash(&mut s);
        let test_hash = s.finish();
        let test_string = format!("Test #{}", test_hash);
        let mut message = vec![0u8; test_spec.message_len];

        pretty_print("LOG", &test_string,
                     &format!("Beginning UDP test with test spec {:?}\n", test_spec),
                     false);


        let durations: Vec<Option<Duration>> =
            (0..test_spec.num_messages)
            .map(|i| self.udp_message(&mut message, &test_string, i))
            .collect();


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
            test: Test::UdpTest(test_spec),
            individual_durations: durations,
            total_duration: total,
        })
    }

    fn tcp_message(&mut self, message: &mut [u8], test_string: &str, message_number: u32) -> Option<Duration> {
        // Copy the current message number (nth message), which is a u32, into the message.
        let message_bytes = unsafe { mem::transmute::<u32, [u8; 4]>(message_number) };
        for p in 0..message.len() {
            message[p] = message_bytes[p & 3];
        }

        // To measure how long it takes to send and receive the message
        let now = Instant::now();

        match self.tcp.write_all(message) {
            Ok(_bytes_sent) => {
                pretty_print("LOG",
                             test_string,
                             &format!("Sent message #{}", message_number),
                             true);
            },
            Err(e) => {
                pretty_print("LOG",
                             test_string,
                             &format!("Failed to send message #{}, encountered error {:?}", message_number, e),
                             false);
                // Failed to send the packet, so there is no duration for this message
                return None;
            }
        };

        match self.tcp_read_exact(message) {
            Ok(()) => {
                // Check if its the same data we sent (all bytes set to i)
                if (0..message.len())
                        .map(|x| message[x] == message_bytes[x & 3])
                        .fold(true, |x, y| x && y) {
                    Some(now.elapsed())
                // It was something else, so lets just ignore it and try again
                } else {
                    None
                }
            },
            Err(Some(e)) => {
                pretty_print("ERR",
                             test_string,
                             &format!("Encountered error {:?} while trying to receive data. Trying again.", e),
                             false);
                None
            },
            Err(None) => {
                pretty_print("ERR",
                             test_string,
                             "Timed out while trying to receive data",
                             false);
                None
            }
        }
    }

    fn run_tcp_test(&mut self, test_spec: TestSpec) -> TestResult {

        let mut s = DefaultHasher::new();
        test_spec.hash(&mut s);
        let test_hash = s.finish();
        let test_string = format!("Test #{}", test_hash);
        let mut message = vec![0u8; test_spec.message_len];

        pretty_print("LOG", &test_string, &format!("Beginning TCP test with test spec {:?}\n", test_spec), false);


        let durations: Vec<Option<Duration>> =
            (0..test_spec.num_messages)
            .map(|i| self.tcp_message(&mut message, &test_string, i))
            .collect();

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
            test: Test::TcpTest(test_spec),
            individual_durations: durations,
            total_duration: total,
        })
    }
}
