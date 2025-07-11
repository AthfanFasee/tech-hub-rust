use std::net::TcpListener;
use moodfeed::run;

#[tokio::main]
async fn main() {
    if let Err(e) = try_main().await {
        eprintln!("Server startup error: {}", e);
    }
}

async fn try_main() -> Result<(), std::io::Error> {
    let listener = TcpListener::bind("127.0.0.1:8000")?;
    run(listener)?.await
}