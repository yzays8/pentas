fn main() {
    if let Err(e) = pentas::run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
