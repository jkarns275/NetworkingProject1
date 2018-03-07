use std::io;
use std::time::Duration;
use std::ops::{ Div, Add };

/// A web test that should use either a TCP/IP connection or a UDP connection. Both contain a
/// TestSpec struct that has specifications for the test.
#[derive(Serialize, Deserialize, Debug, Hash)]
pub enum Test {
    UdpTest(TestSpec),
    TcpTest(TestSpec)
}

/// A struct that has specifications for a test to follow.
#[derive(Hash, Debug, Serialize, Deserialize)]
pub struct TestSpec {
    /// The number of messages that should be sent
    pub num_messages: u32,
    /// The length of the message that should be sent
    pub message_len: usize,
}

/// Return type for a Test being ran
pub type TestResult = Result<TestData, Option<io::Error>>;


/// A structure containing data about a test that ran.
#[derive(Hash, Debug, Serialize, Deserialize)]
pub struct TestData {
    /// The amount of time the test took to finish
    pub total_duration: Duration,

    /// The amount of time each test took to finish in nanoseconds
    pub individual_durations: Vec<Option<Duration>>,

    /// The spec the test followed
    pub test: Test,

    /// The messages that were dropped. The values correspond to the message number
    /// that was dropped.
    pub dropped_messages: Vec<u32>
}

impl TestData {
    pub fn average_duration(&self) -> Duration {
        let mut total = Duration::new(0, 0);
        let mut num_messages = 0;
        for i in self.individual_durations.iter() {
            match i {
                &Some(ref x) => {

                    num_messages += 1;
                    total = total.add(*x);
                },
                &None => {}
            }
        }
        total = total.div(num_messages);
        total
    }
}