use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::Emitter;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Semaphore;
use tokio::time::timeout;

use crate::types::*;

pub fn service_name(port: u16) -> Option<&'static str> {
    match port {
        20 => Some("ftp-data"),
        21 => Some("ftp"),
        22 => Some("ssh"),
        23 => Some("telnet"),
        25 => Some("smtp"),
        53 => Some("domain"),
        67 => Some("dhcps"),
        69 => Some("tftp"),
        80 => Some("http"),
        110 => Some("pop3"),
        111 => Some("rpcbind"),
        119 => Some("nntp"),
        123 => Some("ntp"),
        135 => Some("msrpc"),
        137 => Some("netbios-ns"),
        139 => Some("netbios-ssn"),
        143 => Some("imap"),
        161 => Some("snmp"),
        179 => Some("bgp"),
        389 => Some("ldap"),
        443 => Some("https"),
        445 => Some("microsoft-ds"),
        465 => Some("smtps"),
        514 => Some("syslog"),
        515 => Some("printer"),
        587 => Some("submission"),
        631 => Some("ipp"),
        636 => Some("ldaps"),
        993 => Some("imaps"),
        995 => Some("pop3s"),
        1080 => Some("socks"),
        1433 => Some("ms-sql-s"),
        1521 => Some("oracle"),
        1723 => Some("pptp"),
        1883 => Some("mqtt"),
        2049 => Some("nfs"),
        2181 => Some("zookeeper"),
        2375 => Some("docker"),
        2376 => Some("docker-ssl"),
        3000 => Some("http-alt"),
        3306 => Some("mysql"),
        3389 => Some("ms-wbt-server"),
        3690 => Some("svn"),
        5000 => Some("upnp"),
        5432 => Some("postgresql"),
        5672 => Some("amqp"),
        5900 => Some("vnc"),
        5985 => Some("wsman"),
        5986 => Some("wsmans"),
        6379 => Some("redis"),
        6443 => Some("kubernetes"),
        7001 => Some("weblogic"),
        8009 => Some("ajp"),
        8080 => Some("http-proxy"),
        8443 => Some("https-alt"),
        8888 => Some("jupyter"),
        9090 => Some("zeus-admin"),
        9200 => Some("elasticsearch"),
        10000 => Some("webmin"),
        11211 => Some("memcache"),
        15672 => Some("rabbitmq-mgmt"),
        27017 => Some("mongodb"),
        27018 => Some("mongodb"),
        50000 => Some("db2"),
        _ => None,
    }
}

async fn grab_banner(stream: &mut TcpStream, port: u16) -> Option<(String, String)> {
    let is_http = matches!(
        port,
        80 | 8000 | 8008 | 8080 | 8081 | 8082 | 8083 | 8084 | 8085 | 8088 | 3000 | 9000 | 9090
    );

    if is_http {
        let _ = timeout(
            Duration::from_millis(500),
            stream.write_all(b"HEAD / HTTP/1.0\r\nHost: localhost\r\nConnection: close\r\n\r\n"),
        )
        .await;
    }

    let mut buf = vec![0u8; 512];
    let n = timeout(Duration::from_millis(1000), stream.read(&mut buf))
        .await
        .ok()?.ok()?;

    if n == 0 {
        return None;
    }

    let banner = String::from_utf8_lossy(&buf[..n]);

    // SSH: "SSH-2.0-OpenSSH_9.6p1 Ubuntu-3ubuntu13.15"
    if let Some(line) = banner.lines().find(|l| l.starts_with("SSH-")) {
        let after = line.trim_start_matches("SSH-");
        let parts: Vec<&str> = after.splitn(2, '-').collect();
        if parts.len() == 2 {
            let sw = parts[1];
            if let Some(idx) = sw.find('_') {
                let product = sw[..idx].to_string();
                let version = sw[idx + 1..].split_whitespace().next().unwrap_or("").to_string();
                return Some((product, version));
            }
            return Some((sw.to_string(), String::new()));
        }
    }

    // HTTP response with Server header
    if banner.starts_with("HTTP/") {
        for line in banner.lines() {
            if line.to_lowercase().starts_with("server:") {
                let server = line[7..].trim();
                if let Some(slash) = server.find('/') {
                    let product = server[..slash].trim().to_string();
                    let rest = server[slash + 1..].trim();
                    let version = rest.split_whitespace().next().unwrap_or("").to_string();
                    return Some((product, version));
                }
                return Some((server.to_string(), String::new()));
            }
        }
        return Some(("HTTP".to_string(), String::new()));
    }

    // FTP / SMTP 220 banner
    if banner.starts_with("220") {
        let line = banner.lines().next()?;
        let msg = line[3..].trim().trim_start_matches('-').trim();
        let words: Vec<&str> = msg.split_whitespace().collect();
        if !words.is_empty() {
            let product = words[0].to_string();
            let version = words.get(1).map(|s| s.to_string()).unwrap_or_default();
            return Some((product, version));
        }
    }

    None
}

async fn scan_single_port(
    ip: IpAddr,
    port: u16,
    timeout_ms: u64,
    detect_service: bool,
) -> PortResult {
    let socket_addr = SocketAddr::new(ip, port);

    let connect = timeout(
        Duration::from_millis(timeout_ms),
        TcpStream::connect(socket_addr),
    )
    .await;

    match connect {
        Ok(Ok(mut stream)) => {
            let (product, version) = if detect_service {
                grab_banner(&mut stream, port)
                    .await
                    .map(|(p, v)| (Some(p), Some(v)))
                    .unwrap_or((None, None))
            } else {
                (None, None)
            };

            PortResult {
                port,
                protocol: Protocol::TCP,
                status: PortStatus::Open,
                service: service_name(port).map(|s| s.to_string()),
                product,
                version,
            }
        }
        Ok(Err(_)) => PortResult {
            port,
            protocol: Protocol::TCP,
            status: PortStatus::Closed,
            service: None,
            product: None,
            version: None,
        },
        Err(_) => PortResult {
            port,
            protocol: Protocol::TCP,
            status: PortStatus::Filtered,
            service: None,
            product: None,
            version: None,
        },
    }
}

pub async fn run_scan(app: tauri::AppHandle, config: ScanConfig) -> Result<ScanResult, String> {
    let start_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    // Resolve target to IP address
    let target_addr = format!("{}:0", config.target);
    let ip = tokio::net::lookup_host(&target_addr)
        .await
        .map_err(|e| format!("Failed to resolve '{}': {}", config.target, e))?
        .next()
        .ok_or_else(|| format!("No address found for '{}'", config.target))?
        .ip();

    let resolved_ip = ip.to_string();
    let total = config.ports.len() as u32;
    let scanned = Arc::new(AtomicU32::new(0));
    let semaphore = Arc::new(Semaphore::new(config.concurrency.max(1)));

    let mut handles = Vec::with_capacity(config.ports.len());

    for port in config.ports {
        let sem = semaphore.clone();
        let scanned_ref = scanned.clone();
        let app_clone = app.clone();
        let timeout_ms = config.timeout_ms;
        let detect_service = config.detect_service;

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let result = scan_single_port(ip, port, timeout_ms, detect_service).await;
            let n = scanned_ref.fetch_add(1, Ordering::Relaxed) + 1;

            let _ = app_clone.emit(
                "scan-progress",
                ScanProgress {
                    scanned: n,
                    total,
                    port,
                    is_open: matches!(result.status, PortStatus::Open),
                },
            );

            result
        });

        handles.push(handle);
    }

    let mut results: Vec<PortResult> = futures::future::join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    // Open ports first, then sort by port number
    results.sort_by(|a, b| {
        let a_open = matches!(a.status, PortStatus::Open) as u8;
        let b_open = matches!(b.status, PortStatus::Open) as u8;
        b_open.cmp(&a_open).then(a.port.cmp(&b.port))
    });

    let open_ports: Vec<PortResult> = results
        .into_iter()
        .filter(|r| matches!(r.status, PortStatus::Open))
        .collect();

    let open_count = open_ports.len() as u32;

    let end_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    Ok(ScanResult {
        target: config.target,
        resolved_ip,
        hostname: None,
        os_guess: None,
        start_ms,
        end_ms,
        open_count,
        ports: open_ports,
    })
}
