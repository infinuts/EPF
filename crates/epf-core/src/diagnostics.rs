use crate::{Error, NetworkStatus, Result};
use serde::{Deserialize, Serialize};
use std::{io::Read, net::{IpAddr, Ipv4Addr}, time::Duration};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NatAssessment { Unknown, Cgnat, DoubleNat, Mismatch, Public }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub network: NetworkStatus,
    pub public_ip: Option<IpAddr>,
    pub nat: NatAssessment,
    pub findings: Vec<String>,
    pub reachability: String,
}

pub fn assess(wan: Option<IpAddr>, public: Option<IpAddr>) -> NatAssessment {
    match wan {
        Some(IpAddr::V4(ip)) if is_shared(ip) => NatAssessment::Cgnat,
        Some(IpAddr::V4(ip)) if ip.is_private() => NatAssessment::DoubleNat,
        Some(IpAddr::V4(ip)) if ip.is_unspecified() || ip.is_loopback() || ip.is_link_local()
            || ip.is_multicast() || ip.is_broadcast() => NatAssessment::Unknown,
        Some(wan) => match public {
            Some(public) if wan != public => NatAssessment::Mismatch,
            Some(_) => NatAssessment::Public,
            None => NatAssessment::Unknown,
        },
        None => NatAssessment::Unknown,
    }
}

pub fn is_shared(ip: Ipv4Addr) -> bool {
    let octets = ip.octets(); octets[0] == 100 && (64..=127).contains(&octets[1])
}

/// Only call with the user's consent: this sends an HTTPS request to api4.ipify.org.
pub fn public_ip() -> Result<IpAddr> {
    let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(6))
        .redirect(reqwest::redirect::Policy::none()).build().map_err(network)?;
    let mut body = String::new();
    client.get("https://api4.ipify.org").send().map_err(network)?.error_for_status().map_err(network)?
        .take(65).read_to_string(&mut body).map_err(network)?;
    if body.len() > 64 { return Err(network("The public IP response was too large.")); }
    body.trim().parse().map_err(network)
}

pub fn run(network: NetworkStatus, include_public_ip: bool) -> Diagnostics {
    let mut findings = Vec::new();
    let public = if include_public_ip {
        match public_ip() { Ok(ip) => Some(ip), Err(e) => { findings.push(e.to_string()); None } }
    } else { None };
    let nat = assess(network.wan_ip, public);
    findings.push(match nat {
        NatAssessment::Cgnat => "Your router reports an address in the shared 100.64.0.0/10 range. CGNAT is likely; ask your ISP for a public IPv4 address or use a tunnel.",
        NatAssessment::DoubleNat => "Your router has a private WAN address. A second router or your ISP may be doing NAT. You may need forwarding on the upstream router or a public IPv4 from your ISP.",
        NatAssessment::Mismatch => "The router's WAN address differs from the address seen by ipify. This can happen with upstream NAT, a VPN, or an HTTP proxy. CGNAT is not confirmed.",
        NatAssessment::Public => "The router's WAN address matches the public IP check. This does not prove an individual port is reachable.",
        NatAssessment::Unknown => "There is not enough information to determine upstream NAT. You can optionally compare your WAN address with a public IP check.",
    }.into());
    if !network.upnp_available { findings.push(network.message.clone()); }
    findings.push("Keep the app or server listening on the internal port. Your computer's firewall and ISP can still block incoming connections.".into());
    Diagnostics { network, public_ip: public, nat, findings,
        reachability: "Not tested. No external port-testing service is configured in this release. A router mapping is not proof of internet accessibility; a check from inside your LAN is not an external test.".into() }
}

fn network(e: impl std::fmt::Display) -> Error {
    Error::Network(format!("Could not check your public IP: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ip(s: &str) -> Option<IpAddr> { Some(s.parse().unwrap()) }
    #[test]
    fn nat_findings_do_not_overclaim_reachability() {
        assert_eq!(assess(ip("100.64.0.1"), None), NatAssessment::Cgnat);
        assert_eq!(assess(ip("100.127.255.254"), None), NatAssessment::Cgnat);
        assert!(!is_shared("100.128.0.1".parse().unwrap()));
        assert_eq!(assess(ip("192.168.0.1"), ip("8.8.8.8")), NatAssessment::DoubleNat);
        assert_eq!(assess(ip("8.8.8.8"), ip("1.1.1.1")), NatAssessment::Mismatch);
        assert_eq!(assess(ip("8.8.8.8"), ip("8.8.8.8")), NatAssessment::Public);
        assert_eq!(assess(ip("8.8.8.8"), None), NatAssessment::Unknown);
        assert_eq!(assess(None, ip("8.8.8.8")), NatAssessment::Unknown);
    }
}
