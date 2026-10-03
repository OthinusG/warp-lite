//! Clean stdio endpoint invoked explicitly through verified system SSH.
#[tokio::main]
async fn main() {
    if std::env::args_os().count() != 1 {
        eprintln!("Companion accepts managed control on stdio only");
        std::process::exit(2);
    }
    if warp_agent_bus::companion::serve_stdio().await.is_err() {
        // Peer bytes, native paths and OS diagnostics never enter protocol stderr.
        eprintln!("Companion control channel closed");
        std::process::exit(1);
    }
}
