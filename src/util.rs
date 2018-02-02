use test::*;
use csv::Writer;
use std::fs::File;
use std::io;
use std::net::*;

pub fn create_address(s: &str) -> Result<SocketAddr, ()> {
    let iter = s.to_socket_addrs();
    if iter.is_err() {
        Err(())
    } else if let Some(x) = iter.unwrap().next() {
        Ok(x)
    } else {
        Err(())
    }
}

pub fn pretty_print(subject: &str, key: &str, value: &str, same_line: bool) {
    if same_line {
        println!("\u{001b}[1A[F\r[\u{001b}[32;1m{:<3}\u{001b}[0m] {:<16}: {}", subject, key, value)
    } else {
        println!("[\u{001b}[32;1m{:<3}\u{001b}[0m] {:<16}: {}", subject, key, value)
    }
}

pub fn save_data_as_csv<S: Into<String>>(data: Vec<TestData>, filename: S) -> Result<(), io::Error> {
    let filename = filename.into();
    let file = File::create(filename)?;

    let mut writer = Writer::from_writer(file);
    // Test averages
    writer.write_record(&["Transfer Protocall",
                                "number of messages",
                                "data size (bytes)",
                                "average time (s)",
                                "average throughput (bytes / sec)",
                                "dropped messages"])?;

    for mut test in data.iter() {
        let (data_type, data_size) = match &test.test {
            &Test::UdpTest(ref spec) => ("udp", spec.message_len),
            &Test::TcpTest(ref spec) => ("tcp", spec.message_len),
        };

        let number_of_messages = test.individual_durations.len();

        let average_time = test.average_duration();

        let average_time_double = (average_time.as_secs() as f64) + (average_time.subsec_nanos() as f64 / 1_000_000_000.0f64);

        let dropped_messages = test.dropped_messages.len();

        writer.write_record(&[data_type,
                                    &number_of_messages.to_string(),
                                    &data_size.to_string(),
                                    &average_time_double.to_string(),
                                    // Calculate through put by calculating (messages_sent * message_size) / (average_time * messages_sent)
                                    &(data_size as f64 / average_time_double).to_string(),
                                    &dropped_messages.to_string()])?;
    };

    // Individual data points
    writer.write_record(&["Transfer Protocall", "data size (bytes)", "time (s)", "throughput (bytes / s)", "", ""])?;
    for test in data.iter() {
        let (data_type, data_size_string, data_size) = match &test.test {
            &Test::UdpTest(ref spec) => ("udp", spec.message_len.to_string(), spec.message_len),
            &Test::TcpTest(ref spec) => ("tcp", spec.message_len.to_string(), spec.message_len),
        };

        for dur in test.individual_durations.iter() {
            if let Some(duration) = *dur {
                let dur_double = (duration.as_secs() as f64) + (duration.subsec_nanos() as f64 / 1_000_000_000.0f64);
                writer.write_record(&[data_type,
                                            &data_size_string,
                                            &dur_double.to_string(),
                                            &(data_size as f64 / dur_double).to_string(), "", ""])?;
            }
        }
    }

    writer.flush()?;

    Ok(())
}