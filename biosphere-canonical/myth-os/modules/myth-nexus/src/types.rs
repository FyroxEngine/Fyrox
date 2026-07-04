pub const CRATE_NAME: &str = "myth-nexus";
pub const CREST: &str = "Nexus";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum NetworkRole { Host, Client, Peer, Observer, Relay }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SyncStrategy { FullState, DeltaOnly, EventOnly, Manual }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ExternalSourceType { Sensor, Api, WebSocket, TcpStream, UdpStream, FileWatch, Mqtt }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExternalSource {
    pub source_id: String,
    pub label: String,
    pub source_type: ExternalSourceType,
    pub endpoint: String,           // URL, IP:port, file path, MQTT topic
    pub poll_rate_ms: u64,
    pub auth_token: Option<String>,
    pub wire_type_out: String,      // 3-char WireType code for incoming data
    pub transform_fn: Option<String>, // optional named transform to apply
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NexusConfig {
    pub network_role: NetworkRole,
    pub listen_addr: String,        // "127.0.0.1:7700"
    pub max_peers: u8,              // cap 16 (Law of 16)
    pub sync_strategy: SyncStrategy,
    pub sync_rate_hz: u32,
    pub compression: bool,
    pub encryption: bool,
    pub external_sources: Vec<ExternalSource>,  // sensors, APIs, websockets
    pub heartbeat_interval_ms: u64,
    pub timeout_ms: u64,
    pub allow_observer_connections: bool,
    pub packet_buffer_size: u32,
}

impl Default for NexusConfig {
    fn default() -> Self {
        Self {
            network_role: NetworkRole::Host,
            listen_addr: "127.0.0.1:7700".into(),
            max_peers: 16,
            sync_strategy: SyncStrategy::DeltaOnly,
            sync_rate_hz: 20,
            compression: true,
            encryption: false,
            external_sources: vec![],
            heartbeat_interval_ms: 1000,
            timeout_ms: 5000,
            allow_observer_connections: true,
            packet_buffer_size: 1024,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RemoteEvent {
    pub peer_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub received_at: f64,
}
