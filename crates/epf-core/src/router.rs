use crate::{is_destination, now, Error, NetworkStatus, Protocol, Result, Rule};
use igd_next::{Gateway, GetGenericPortMappingEntryError, PortMappingProtocol, SearchOptions};
use std::{
    io::Read,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::{Duration, Instant},
};

#[derive(Debug, Clone)]
pub struct Mapping {
    pub external_port: u16,
    pub internal_port: u16,
    pub protocol: Protocol,
    pub destination_ip: Ipv4Addr,
    pub description: String,
    pub enabled: bool,
    pub remote_host: String,
}

impl Mapping {
    pub fn matches(&self, rule: &Rule) -> bool {
        self.external_port == rule.external_port && self.protocol == rule.protocol
    }

    pub fn owned_by(&self, rule: &Rule) -> bool {
        self.matches(rule)
            && self.internal_port == rule.internal_port
            && self.destination_ip == rule.destination_ip
            && self.description == rule.description
            && self.remote_host.is_empty()
    }
}

/// Implement this trait to add another protocol without duplicating rule management.
pub trait Router {
    fn id(&self) -> String;
    fn local_ip(&self) -> Ipv4Addr;
    fn accepts_destination(&self, ip: Ipv4Addr) -> bool;
    fn mappings(&mut self) -> Result<Vec<Mapping>>;
    /// Inspect only the keys involved in an operation. Existing adapters keep
    /// their enumeration behavior unless they support a targeted query.
    fn inspect(&mut self, _keys: &[(u16, Protocol)]) -> Result<Vec<Mapping>> {
        self.mappings()
    }
    fn add(&mut self, rule: &Rule) -> Result<()>;
    fn remove(&mut self, rule: &Rule) -> Result<()>;
}

pub struct UpnpRouter {
    gateway: Gateway,
    local_ip: Ipv4Addr,
    subnets: Vec<netdev::ipnet::Ipv4Net>,
}

impl UpnpRouter {
    pub fn discover() -> (NetworkStatus, Option<Self>) {
        let mut status = NetworkStatus {
            local_ip: None,
            gateway_ip: None,
            interface: None,
            router_name: None,
            manufacturer: None,
            model: None,
            router_id: None,
            wan_ip: None,
            upnp_available: false,
            message: String::new(),
            checked_at: now(),
        };
        let interface = match netdev::get_default_interface() {
            Ok(interface) => interface,
            Err(e) => {
                status.message = format!("Could not find a default network interface: {e}. Connect to Wi-Fi or Ethernet and try again.");
                return (status, None);
            }
        };
        status.interface = Some(
            interface
                .friendly_name
                .clone()
                .unwrap_or(interface.name.clone()),
        );
        status.gateway_ip = interface
            .gateway
            .as_ref()
            .and_then(|g| g.ipv4.first().copied());
        let local_ip = match interface
            .ipv4
            .iter()
            .map(|n| n.addr())
            .find(|ip| is_destination(*ip))
        {
            Some(ip) => ip,
            None => {
                status.message =
                    "This connection has no usable IPv4 address. The MVP needs an IPv4 network."
                        .into();
                return (status, None);
            }
        };
        status.local_ip = Some(local_ip);
        let mut options = SearchOptions::default();
        options.bind_addr = SocketAddr::new(IpAddr::V4(local_ip), 0);
        options.timeout = Some(Duration::from_secs(4));
        options.single_search_timeout = Some(Duration::from_secs(1));
        let gateway = match igd_next::search_gateway(options)
            .and_then(|gateway| {
                if status
                    .gateway_ip
                    .is_some_and(|ip| gateway.addr.ip() != IpAddr::V4(ip))
                {
                    Err(igd_next::SearchError::InvalidResponse)
                } else {
                    Ok(gateway)
                }
            })
            .or_else(|original| {
                let Some(ip) = status.gateway_ip else {
                    return Err(original);
                };
                let mut directed = SearchOptions::default();
                directed.bind_addr = SocketAddr::new(IpAddr::V4(local_ip), 0);
                directed.broadcast_address = SocketAddr::new(IpAddr::V4(ip), 1900);
                directed.timeout = Some(Duration::from_secs(3));
                directed.single_search_timeout = Some(Duration::from_secs(1));
                igd_next::search_gateway(directed)
            }) {
            Ok(gateway) => gateway,
            Err(e) => {
                status.message = format!("No UPnP router responded: {e}. Check that UPnP is enabled, connect directly to your home network, and try without a VPN. Your router may not support automatic forwarding.");
                return (status, None);
            }
        };
        if status
            .gateway_ip
            .is_some_and(|ip| gateway.addr.ip() != IpAddr::V4(ip))
        {
            status.message = "A UPnP device responded, but it is not the default gateway. EPF will not change it. Check for a VPN or a second router.".into();
            return (status, None);
        }
        // UPnP advertises local HTTP endpoints. igd-next validates device/service hosts.
        if let Ok(metadata) = router_metadata(&gateway) {
            (status.router_name, status.manufacturer, status.model) = metadata;
        }
        status.wan_ip = gateway.get_external_ip().ok();
        let router = Self {
            gateway,
            local_ip,
            subnets: interface.ipv4,
        };
        status.router_id = Some(router.id());
        status.upnp_available = true;
        status.message = "Your router supports UPnP. You can request a forwarding rule.".into();
        (status, Some(router))
    }
}

impl Router for UpnpRouter {
    fn id(&self) -> String {
        format!(
            "{}|{}|{}",
            self.gateway.addr, self.gateway.control_url, self.gateway.service_type
        )
    }
    fn local_ip(&self) -> Ipv4Addr {
        self.local_ip
    }
    fn accepts_destination(&self, ip: Ipv4Addr) -> bool {
        is_destination(ip)
            && self.subnets.iter().any(|subnet| {
                subnet.contains(&ip) && ip != subnet.network() && ip != subnet.broadcast()
            })
    }
    fn mappings(&mut self) -> Result<Vec<Mapping>> {
        let start = Instant::now();
        let mut mappings = Vec::new();
        for index in 0..4096 {
            if start.elapsed() > Duration::from_secs(20) {
                return Err(Error::Network(
                    "Reading router rules took too long. EPF has not changed them. Try again."
                        .into(),
                ));
            }
            match self.gateway.get_generic_port_mapping_entry(index) {
                Ok(entry) => {
                    // A hostname cannot be compared safely to a saved IPv4 destination.
                    // Keep its occupied key, but never treat it as an owned mapping.
                    mappings.push(Mapping {
                        external_port: entry.external_port, internal_port: entry.internal_port,
                        protocol: match entry.protocol { PortMappingProtocol::TCP => Protocol::Tcp, PortMappingProtocol::UDP => Protocol::Udp },
                        destination_ip: entry.internal_client.parse().unwrap_or(Ipv4Addr::UNSPECIFIED),
                        description: entry.port_mapping_description, enabled: entry.enabled,
                        remote_host: entry.remote_host,
                    });
                }
                Err(GetGenericPortMappingEntryError::SpecifiedArrayIndexInvalid) => return Ok(mappings),
                Err(e) => return Err(Error::Network(format!("Could not inspect existing router rules: {e}. EPF needs this check to avoid changing someone else's rule."))),
            }
        }
        Err(Error::Network(
            "The router returned too many rules to inspect safely.".into(),
        ))
    }
    fn inspect(&mut self, keys: &[(u16, Protocol)]) -> Result<Vec<Mapping>> {
        match crate::upnp_query::inspect(&self.gateway, self.local_ip, keys)? {
            Some(mappings) => Ok(mappings),
            None => self.mappings(),
        }
    }
    fn add(&mut self, rule: &Rule) -> Result<()> {
        self.gateway.add_port(to_igd(rule.protocol)?, rule.external_port,
            SocketAddr::new(IpAddr::V4(rule.destination_ip), rule.internal_port), rule.lease_seconds, &rule.description)
            .map_err(|e| Error::Network(format!("The router did not confirm {} port {}: {e}. Some routers require manual rules or reject temporary leases.", rule.protocol, rule.external_port)))
    }
    fn remove(&mut self, rule: &Rule) -> Result<()> {
        self.gateway.remove_port(to_igd(rule.protocol)?, rule.external_port)
            .map_err(|e| Error::Network(format!("The router did not confirm removing {} port {}: {e}. Refresh to check its current state.", rule.protocol, rule.external_port)))
    }
}

fn to_igd(protocol: Protocol) -> Result<PortMappingProtocol> {
    match protocol {
        Protocol::Tcp => Ok(PortMappingProtocol::TCP),
        Protocol::Udp => Ok(PortMappingProtocol::UDP),
        Protocol::Both => Err(Error::Validation(
            "Each router mapping must use one protocol.".into(),
        )),
    }
}

type Metadata = (Option<String>, Option<String>, Option<String>);
fn router_metadata(gateway: &Gateway) -> std::result::Result<Metadata, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(3))
        .build()?;
    let url = format!("http://{}{}", gateway.addr, gateway.root_url);
    let mut text = String::new();
    client
        .get(url)
        .send()?
        .error_for_status()?
        .take(262145)
        .read_to_string(&mut text)?;
    if text.len() > 262144 {
        return Err("Device description is too large".into());
    }
    let xml = roxmltree::Document::parse(&text)?;
    let field = |name| {
        xml.descendants()
            .find(|n| n.has_tag_name(name))
            .and_then(|n| n.text())
            .map(|s| s.chars().take(128).collect())
    };
    Ok((
        field("friendlyName"),
        field("manufacturer"),
        field("modelName"),
    ))
}
