mod server;
mod test;
mod util;

use std::env;

const USAGE_MESSAGE: &'static str = r#"
Usage: dl1 [mode]

modes: serve, echo

"#;

const ECHO: &'static str = "echo";
const SERVE: &'static str = "serve";

fn main() {
    let args = env::args().into_iter().collect::<Vec<String>>();

    if args.len() == 1 {
        println!("{}", USAGE_MESSAGE);
    } else if (&args[1] == ECHO) {
        server::Server::echo().unwrap();
    } else if (&args[1] == SERVE) {
        let mut server;
        match server::Server::new() {
            Ok(s) => server = s,
            Err(e) => {
                println!("Encountered error '{}' while trying to create server.", e);
                return
            }
        };

        let result = server.run_tests(
            vec![
                test::Test::UdpTest(test::TestSpec { num_messages: 8, message_len: 1024 }),
                test::Test::UdpTest(test::TestSpec { num_messages: 8, message_len: 65504 }),
            ]
        );

        println!("{:?}", result);

        println!("Successfully created server")
    } else {
        println!("{}", USAGE_MESSAGE);
    }
}
