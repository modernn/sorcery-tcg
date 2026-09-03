use std::io;

use sorcery_engine::session_json::SessionJsonService;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), sorcery_engine::session_json::SessionJsonError> {
    let stdin = io::stdin();
    let mut service = SessionJsonService::new();
    service.serve(stdin.lock(), io::stdout())
}
