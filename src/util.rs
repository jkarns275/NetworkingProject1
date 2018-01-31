pub fn pretty_print(subject: &str, key: &str, value: &str, same_line: bool) {
    if same_line {
        println!("\u{001b}[1A[F\r[\u{001b}[32;1m{:<3}\u{001b}[0m] {:<16}: {}", subject, key, value)
    } else {
        println!("[\u{001b}[32;1m{:<3}\u{001b}[0m] {:<16}: {}", subject, key, value)
    }
}