// CORE-ATOM-03: Socket Manager — IPC connection management (stub; full gRPC/UDS in next phase)
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
