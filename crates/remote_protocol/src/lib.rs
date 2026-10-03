//! Shared SSH control wire types and framing, independent of the desktop app.
pub mod protocol;
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/remote_server.rs"));
}
