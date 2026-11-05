use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Tcp,
    Udp,
    Both,
}

impl Protocol {
    pub fn expand(self) -> Vec<Self> {
        match self {
            Self::Both => vec![Self::Tcp, Self::Udp],
            protocol => vec![protocol],
        }
    }
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Tcp => "TCP",
            Self::Udp => "UDP",
            Self::Both => "TCP + UDP",
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenRequest {
    pub name: String,
    pub external_port: u16,
    pub internal_port: u16,
    pub protocol: Protocol,
    /// None means the detected local IPv4 address.
    pub destination_ip: Option<Ipv4Addr>,
    pub lease_seconds: u32,
}

impl OpenRequest {
    pub fn validate(&self) -> Result<()> {
        if self.external_port == 0 || self.internal_port == 0 {
            return Err(Error::Validation("Choose a port between 1 and 65535.".into()));
        }
        if self.name.trim().is_empty() || self.name.len() > 64 || self.name.chars().any(char::is_control) {
            return Err(Error::Validation("Give this rule a name of 1 to 64 characters, without control characters.".into()));
        }
        if !(60..=86400).contains(&self.lease_seconds) {
            return Err(Error::Validation("Choose a lease between 60 seconds and 24 hours.".into()));
        }
        if let Some(ip) = self.destination_ip {
            if !is_destination(ip) {
                return Err(Error::Validation("Use a unicast IPv4 address for the destination.".into()));
            }
        }
        Ok(())
    }
}

pub fn is_destination(ip: Ipv4Addr) -> bool {
    !ip.is_unspecified() && !ip.is_loopback() && !ip.is_multicast() && !ip.is_broadcast()
        && !ip.is_link_local() && ip.octets()[0] != 0 && ip.octets()[0] < 224
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleStatus {
    Pending,
    Active,
    Inactive,
    Conflict,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub external_port: u16,
    pub internal_port: u16,
    pub protocol: Protocol,
    pub destination_ip: Ipv4Addr,
    pub lease_seconds: u32,
    pub created_at: u64,
    pub router_id: String,
    pub description: String,
    pub status: RuleStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub request: OpenRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub schema_version: u32,
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub profiles: Vec<Profile>,
}

impl Default for Config {
    fn default() -> Self {
        Self { schema_version: 1, rules: Vec::new(), profiles: Vec::new() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStatus {
    pub local_ip: Option<Ipv4Addr>,
    pub gateway_ip: Option<Ipv4Addr>,
    pub interface: Option<String>,
    pub router_name: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub router_id: Option<String>,
    pub wan_ip: Option<std::net::IpAddr>,
    pub upnp_available: bool,
    pub message: String,
    pub checked_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub network: NetworkStatus,
    pub rules: Vec<Rule>,
    pub profiles: Vec<Profile>,
    pub warning: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn request() -> OpenRequest {
        OpenRequest { name: "Minecraft".into(), external_port: 25565, internal_port: 25565,
            protocol: Protocol::Both, destination_ip: None, lease_seconds: 3600 }
    }

    #[test]
    fn validates_ports_addresses_names_and_leases() {
        let mut req = request();
        assert!(req.validate().is_ok());
        req.external_port = 0;
        assert!(req.validate().is_err());
        req.external_port = 65535;
        for ip in ["127.0.0.1", "0.0.0.0", "255.255.255.255", "224.0.0.1", "169.254.1.1", "0.1.2.3", "240.1.2.3"] {
            req.destination_ip = Some(ip.parse().unwrap());
            assert!(req.validate().is_err(), "{ip}");
        }
        req.destination_ip = Some("192.168.1.25".parse().unwrap());
        assert!(req.validate().is_ok());
        req.lease_seconds = 0;
        assert!(req.validate().is_err());
        req.lease_seconds = 3600;
        req.name = "bad\nname".into();
        assert!(req.validate().is_err());
    }
}
