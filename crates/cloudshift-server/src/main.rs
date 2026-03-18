//! CloudShift HTTP server entrypoint.

#[tokio::main]
async fn main() {
    if let Err(e) = cloudshift_server::run().await {
        eprintln!("cloudshift-server: {e:#}");
        std::process::exit(1);
    }
}
