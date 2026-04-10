use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Protocol {
    #[serde(rename = "TCP")]
    TCP,
    #[serde(rename = "UDP")]
    UDP,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PortStatus {
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "closed")]
    Closed,
    #[serde(rename = "filtered")]
    Filtered,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PortResult {
    pub port: u16,
    pub protocol: Protocol,
    pub status: PortStatus,
    pub service: Option<String>,
    pub product: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanConfig {
    pub target: String,
    pub ports: Vec<u16>,
    pub protocol: String,
    pub detect_service: bool,
    pub detect_os: bool,
    pub timeout_ms: u64,
    pub concurrency: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub scanned: u32,
    pub total: u32,
    pub port: u16,
    pub is_open: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub target: String,
    pub resolved_ip: String,
    pub hostname: Option<String>,
    pub os_guess: Option<String>,
    pub start_ms: u64,
    pub end_ms: u64,
    pub open_count: u32,
    pub ports: Vec<PortResult>,
}
