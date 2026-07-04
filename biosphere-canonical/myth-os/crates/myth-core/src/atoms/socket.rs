// CORE-ATOM-03: Socket Manager — TCP connection lifecycle management.
//
// Stub for the initial build. Full implementation follows in the Theater
// transport layer (biospark-theater crate). For now this holds the bind
// address and confirms configuration at boot.

use tracing::info;

pub struct SocketManager {
    pub bind_addr: String,
}

impl SocketManager {
    pub fn new(bind_addr: impl Into<String>) -> Self {
        let addr = bind_addr.into();
        info!(addr = %addr, "SocketManager configured");
        Self { bind_addr: addr }
    }
}
