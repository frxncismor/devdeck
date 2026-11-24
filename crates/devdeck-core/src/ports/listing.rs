//! Port -> PID listing. The system layer is behind `PortSource` so the logic is testable with a
//! simulated source (constitution II).

use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Tcp,
    Udp,
}

/// One bound socket owned by a process, before enrichment.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RawListener {
    pub port: u16,
    pub protocol: Protocol,
    pub pid: u32,
    /// Name as reported by the socket table (fallback when the process is not visible).
    pub name: String,
}

pub trait PortSource {
    /// Listening TCP sockets and, when `include_udp`, bound UDP sockets. Unavailable => empty.
    fn listeners(&self, include_udp: bool) -> Vec<RawListener>;
}

/// Real implementation over the `listeners` crate (Windows/macOS/Linux; best effort off Windows).
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemPorts;

impl PortSource for SystemPorts {
    fn listeners(&self, include_udp: bool) -> Vec<RawListener> {
        let Ok(all) = listeners::get_all() else {
            return Vec::new();
        };
        let raw = all.into_iter().filter_map(|l| {
            let protocol = match l.protocol {
                listeners::Protocol::TCP if l.state == listeners::SocketState::Listen => {
                    Protocol::Tcp
                }
                listeners::Protocol::UDP if include_udp => Protocol::Udp,
                _ => return None,
            };
            Some(RawListener {
                port: l.socket.port(),
                protocol,
                pid: l.process.pid,
                name: l.process.name,
            })
        });
        dedup(raw.collect())
    }
}

/// Sorts and removes duplicates (the same port is usually bound on both IPv4 and IPv6).
pub fn dedup(items: Vec<RawListener>) -> Vec<RawListener> {
    items
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(port: u16, pid: u32) -> RawListener {
        RawListener {
            port,
            protocol: Protocol::Tcp,
            pid,
            name: "node".into(),
        }
    }

    #[test]
    fn dedup_removes_ipv4_ipv6_duplicates_and_sorts() {
        let v = dedup(vec![raw(5173, 2), raw(3000, 1), raw(5173, 2)]);
        assert_eq!(v, vec![raw(3000, 1), raw(5173, 2)]);
    }

    #[test]
    fn protocol_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&Protocol::Udp).unwrap(), "\"udp\"");
    }
}
