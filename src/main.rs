#[macro_use]
extern crate serde_derive;

extern crate serde;

extern crate serde_json;

mod server;
mod test;
mod util;

use test::*;

use util::pretty_print;

use std::env;
use std::fs::File;
use std::io::Write;

const USAGE_MESSAGE: &'static str = r#"
Usage: dl1 [mode] [tests]

modes: serve, echo, help

test format (keep quotes): "[UDP|TCP] [num_messages] [message_len]"

"#;

const ECHO: &'static str = "echo";
const TEST: &'static str = "test";
const REQ_DATA: &'static str = "required";

fn main() {
    let mut args = env::args().into_iter().collect::<Vec<String>>();

    if args.len() == 1 {
        println!("{}", USAGE_MESSAGE);
    } else if (&args[1] == ECHO) {
        server::Server::echo().unwrap();
    } else if (&args[1] == TEST) {
        let mut server;
        match server::Server::new() {
            Ok(s) => server = s,
            Err(e) => {
                println!("Encountered error '{}' while trying to create server.", e);
                return
            }
        };

        if args.len() < 3 {
            pretty_print("LOG", "Program Argument", &format!("No tests provided."), false);
            return
        }

        let mut tests: Vec<test::Test> = Vec::with_capacity(args.len() - 2);
        for arg in &mut args[2..] {
            arg.replace("\"", "");
            let tokens: Vec<String> = arg.split(' ').map(|s| s.to_lowercase()).collect();
            if tokens.len() < 3 {
                pretty_print("ERR", "Program Argument", &format!("'{}' is not a valid test.", arg), false);
                continue
            }
            let mut num_messages = 0;
            if let Ok(val) = tokens[1].parse::<u32>() {
                num_messages += val;
            } else {
                pretty_print("ERR", "Program Argument", &format!("'{}' is not a valid number.", tokens[1]), false);
                continue
            }

            let mut message_len = 0;
            if let Ok(val) = tokens[2].parse::<usize>() {
                message_len += val
            } else {
                pretty_print("ERR", "Program Argument", &format!("'{}' is not a valid number.", tokens[2]), false);
                continue
            }

            if &tokens[0] == "udp" {
                tests.push(test::Test::UdpTest(test::TestSpec {
                    num_messages,
                    message_len,
                }));
            } else if &tokens[0] == "tcp" {
                tests.push(test::Test::TcpTest(test::TestSpec {
                    num_messages,
                    message_len,
                }));
            } else {
                pretty_print("ERR", "Program Argument", &format!("'{}' is not a valid connection type (tcp or udp only).", tokens[1]), false);
            }
        }

        let result = server.run_tests(tests);

        //println!("{:?}", result);

        println!("Successfully created server")
    } else if &args[1] == REQ_DATA {
        let mut server;
        match server::Server::new() {
            Ok(s) => server = s,
            Err(e) => {
                println!("Encountered error '{}' while trying to create server.", e);
                return
            }
        };

        let result: Vec<test::TestData> = server.run_tests(vec![
            Test::TcpTest(TestSpec { message_len: 1, num_messages: 64 }),
            Test::TcpTest(TestSpec { message_len: 64, num_messages: 64 }),
            Test::TcpTest(TestSpec { message_len: 1024, num_messages: 64 }),
            Test::UdpTest(TestSpec { message_len: 1, num_messages: 64 }),
            Test::UdpTest(TestSpec { message_len: 64, num_messages: 64 }),
            Test::UdpTest(TestSpec { message_len: 1024, num_messages: 64 }),
            Test::TcpTest(TestSpec { message_len: 1024, num_messages: 64 }),
            Test::TcpTest(TestSpec { message_len: 1024 * 16, num_messages: 64 }),
            Test::TcpTest(TestSpec { message_len: 1024 * 64, num_messages: 64 }),
            Test::TcpTest(TestSpec { message_len: 1024 * 256, num_messages: 64 }),
            Test::TcpTest(TestSpec { message_len: 1024 * 1024, num_messages: 64 }),
        ]).unwrap().into_iter().map(Result::ok).filter_map(|x| x).collect();

        let mut file = File::create("output_spectre_to_nuc.json").unwrap();
        let serialized = serde_json::to_string_pretty(&result).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();
    } else {
        println!("{}", USAGE_MESSAGE);
    }
}
