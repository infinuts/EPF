//! Targeted UPnP reads for gateways without working table enumeration.
//! Protocol reference: UPnP WANIPConnection:1, section GetSpecificPortMappingEntry.
use crate::{router::Mapping, Error, Protocol, Result};
use igd_next::Gateway;
use std::{
    io::Read,
    net::Ipv4Addr,
    time::{Duration, Instant},
};

const MAX_BODY: u64 = 65536;
enum Reply {
    Present(Mapping),
    Absent,
    Unsupported,
}

/// None means the router explicitly lacks this action; try legacy enumeration.
pub(crate) fn inspect(
    gateway: &Gateway,
    local_ip: Ipv4Addr,
    keys: &[(u16, Protocol)],
) -> Result<Option<Vec<Mapping>>> {
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .local_address(std::net::IpAddr::V4(local_ip))
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|e| Error::Network(e.to_string()))?;
    let start = Instant::now();
    let mut seen = Vec::new();
    let mut mappings = Vec::new();
    for &(port, protocol) in keys {
        if seen.contains(&(port, protocol)) {
            continue;
        }
        seen.push((port, protocol));
        if start.elapsed() > Duration::from_secs(20) {
            return Err(failure("Reading port status took too long"));
        }
        let body = request(&gateway.service_type, port, protocol)?;
        let response = client
            .post(format!("http://{}{}", gateway.addr, gateway.control_url))
            .header("Content-Type", "text/xml; charset=\"utf-8\"")
            .header(
                "SOAPAction",
                format!("\"{}#GetSpecificPortMappingEntry\"", gateway.service_type),
            )
            .body(body)
            .send()
            .map_err(|e| failure(&e.to_string()))?;
        let success = response.status().is_success();
        let mut xml = String::new();
        response
            .take(MAX_BODY + 1)
            .read_to_string(&mut xml)
            .map_err(|e| failure(&e.to_string()))?;
        if xml.len() as u64 > MAX_BODY {
            return Err(failure("Router response was too large"));
        }
        match parse(&xml, &gateway.service_type, port, protocol, success)? {
            Reply::Present(mapping) => mappings.push(mapping),
            Reply::Absent => (),
            Reply::Unsupported => return Ok(None),
        }
    }
    Ok(Some(mappings))
}

fn failure(detail: &str) -> Error {
    Error::Network(format!(
        "Could not inspect this router port: {detail}. EPF left router rules unchanged."
    ))
}

fn request(service: &str, port: u16, protocol: Protocol) -> Result<String> {
    if ![
        "urn:schemas-upnp-org:service:WANIPConnection:1",
        "urn:schemas-upnp-org:service:WANIPConnection:2",
        "urn:schemas-upnp-org:service:WANPPPConnection:1",
    ]
    .contains(&service)
        || port == 0
        || protocol == Protocol::Both
    {
        return Err(failure("Unsupported service or invalid mapping key"));
    }
    Ok(format!(
        r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/"><s:Body><u:GetSpecificPortMappingEntry xmlns:u="{service}"><NewRemoteHost></NewRemoteHost><NewExternalPort>{port}</NewExternalPort><NewProtocol>{protocol}</NewProtocol></u:GetSpecificPortMappingEntry></s:Body></s:Envelope>"#
    ))
}

fn parse(
    xml: &str,
    service: &str,
    port: u16,
    protocol: Protocol,
    http_success: bool,
) -> Result<Reply> {
    let doc = roxmltree::Document::parse(xml).map_err(|_| failure("Invalid XML response"))?;
    let body = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name(("http://schemas.xmlsoap.org/soap/envelope/", "Body")))
        .ok_or_else(|| failure("Missing SOAP body"))?;
    if let Some(fault) = body
        .children()
        .find(|n| n.has_tag_name(("http://schemas.xmlsoap.org/soap/envelope/", "Fault")))
    {
        let code = fault
            .descendants()
            .find(|n| n.is_element() && n.tag_name().name() == "errorCode")
            .and_then(|n| n.text())
            .unwrap_or("")
            .trim();
        return match code {
            "714" => Ok(Reply::Absent),
            "401" => Ok(Reply::Unsupported),
            _ => Err(failure(&format!("UPnP fault {code}"))),
        };
    }
    if !http_success {
        return Err(failure("Router returned an HTTP error"));
    }
    let response = body
        .children()
        .find(|n| n.has_tag_name((service, "GetSpecificPortMappingEntryResponse")))
        .ok_or_else(|| failure("Missing mapping response"))?;
    let field = |name| -> Result<&str> {
        let mut nodes = response.children().filter(|n| {
            n.is_element()
                && n.tag_name().name() == name
                && (n.tag_name().namespace().is_none() || n.tag_name().namespace() == Some(service))
        });
        let node = nodes
            .next()
            .ok_or_else(|| failure(&format!("Missing {name}")))?;
        if nodes.next().is_some() {
            return Err(failure("Ambiguous mapping response"));
        }
        Ok(node.text().unwrap_or(""))
    };
    let internal_port = field("NewInternalPort")?
        .trim()
        .parse::<u16>()
        .map_err(|_| failure("Invalid internal port"))?;
    if internal_port == 0 {
        return Err(failure("Invalid internal port"));
    }
    let destination_ip = field("NewInternalClient")?
        .trim()
        .parse::<Ipv4Addr>()
        .map_err(|_| failure("Invalid destination address"))?;
    let enabled = match field("NewEnabled")?.trim() {
        "1" | "true" => true,
        "0" | "false" => false,
        _ => return Err(failure("Invalid enabled state")),
    };
    Ok(Reply::Present(Mapping {
        external_port: port,
        internal_port,
        protocol,
        destination_ip,
        description: field("NewPortMappingDescription")?.into(),
        enabled,
        remote_host: String::new(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    const IP1: &str = "urn:schemas-upnp-org:service:WANIPConnection:1";
    const IP2: &str = "urn:schemas-upnp-org:service:WANIPConnection:2";
    const PPP: &str = "urn:schemas-upnp-org:service:WANPPPConnection:1";

    fn envelope(content: &str) -> String {
        format!(
            r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body>{content}</s:Body></s:Envelope>"#
        )
    }
    fn success(service: &str) -> String {
        envelope(&format!(
            r#"<GetSpecificPortMappingEntryResponse xmlns="{service}"><NewInternalPort>25566</NewInternalPort><NewInternalClient>192.168.1.2</NewInternalClient><NewEnabled>1</NewEnabled><NewPortMappingDescription>EPF:test&amp;value</NewPortMappingDescription><NewLeaseDuration>3600</NewLeaseDuration></GetSpecificPortMappingEntryResponse>"#
        ))
    }
    fn fault(code: u16) -> String {
        envelope(&format!(
            r#"<s:Fault><faultcode>s:Client</faultcode><detail><UPnPError xmlns="urn:schemas-upnp-org:control-1-0"><errorCode>{code}</errorCode></UPnPError></detail></s:Fault>"#
        ))
    }
    #[test]
    fn supports_ip_v1_v2_and_ppp_without_losing_ownership_fields() {
        for service in [IP1, IP2, PPP] {
            assert!(request(service, 25565, Protocol::Tcp)
                .unwrap()
                .contains(service));
            let Reply::Present(mapping) =
                parse(&success(service), service, 25565, Protocol::Tcp, true).unwrap()
            else {
                panic!("expected mapping")
            };
            assert_eq!(mapping.external_port, 25565);
            assert_eq!(mapping.internal_port, 25566);
            assert_eq!(mapping.destination_ip, Ipv4Addr::new(192, 168, 1, 2));
            assert_eq!(mapping.description, "EPF:test&value");
            assert!(mapping.enabled && mapping.remote_host.is_empty());
        }
    }
    #[test]
    fn only_explicit_no_such_entry_means_absent() {
        assert!(matches!(
            parse(&fault(714), IP1, 80, Protocol::Tcp, false),
            Ok(Reply::Absent)
        ));
        assert!(matches!(
            parse(&fault(401), IP1, 80, Protocol::Tcp, false),
            Ok(Reply::Unsupported)
        ));
        for code in [402, 501, 606, 713, 718, 725] {
            assert!(parse(&fault(code), IP1, 80, Protocol::Tcp, false).is_err());
        }
        assert!(parse("", IP1, 80, Protocol::Tcp, false).is_err());
        assert!(parse(&success(IP1), IP1, 80, Protocol::Tcp, false).is_err());
    }
    #[test]
    fn incomplete_ambiguous_and_wrong_service_responses_fail_closed() {
        for bad in [
            success(IP1).replace("<NewInternalPort>25566</NewInternalPort>", ""),
            success(IP1).replace("25566", "0"),
            success(IP1).replace("192.168.1.2", "router.local"),
            success(IP1).replace(
                "<NewEnabled>1</NewEnabled>",
                "<NewEnabled>1</NewEnabled><NewEnabled>0</NewEnabled>",
            ),
            success(IP1).replace(
                "<NewEnabled>1</NewEnabled>",
                "<NewEnabled>unknown</NewEnabled>",
            ),
            success(PPP),
        ] {
            assert!(parse(&bad, IP1, 80, Protocol::Tcp, true).is_err());
        }
        assert!(request("untrusted\"service", 80, Protocol::Tcp).is_err());
        assert!(request(IP1, 0, Protocol::Tcp).is_err());
        assert!(request(IP1, 80, Protocol::Both).is_err());
    }
    #[test]
    fn http_queries_are_targeted_deduplicated_and_use_the_advertised_service() {
        use std::{
            io::{BufRead, BufReader, Write},
            net::TcpListener,
        };
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = server.local_addr().unwrap();
        let worker = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for reply in [success(PPP), fault(714)] {
                let (mut stream, _) = server.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut reader = BufReader::new(&mut stream);
                let mut headers = String::new();
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    headers.push_str(&line);
                }
                let length: usize = headers
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse().unwrap())
                    })
                    .unwrap();
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                requests.push((headers, String::from_utf8(body).unwrap()));
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    reply.len(),
                    reply
                )
                .unwrap();
            }
            requests
        });
        let gateway = Gateway {
            addr,
            root_url: "/root.xml".into(),
            control_url: "/control".into(),
            control_schema_url: "/schema".into(),
            control_schema: Default::default(),
            service_type: PPP.into(),
        };
        let entries = inspect(
            &gateway,
            Ipv4Addr::LOCALHOST,
            &[
                (25565, Protocol::Tcp),
                (25565, Protocol::Tcp),
                (2456, Protocol::Udp),
            ],
        )
        .unwrap()
        .unwrap();
        assert_eq!(entries.len(), 1);
        let requests = worker.join().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].0.starts_with("POST /control HTTP/1.1"));
        assert!(requests[0]
            .0
            .contains("WANPPPConnection:1#GetSpecificPortMappingEntry"));
        assert!(requests[0].1.contains("<NewProtocol>TCP</NewProtocol>"));
        assert!(requests[1]
            .1
            .contains("<NewExternalPort>2456</NewExternalPort>"));
        assert!(!requests
            .iter()
            .any(|r| r.1.contains("AddPortMapping") || r.1.contains("DeletePortMapping")));
    }
}
