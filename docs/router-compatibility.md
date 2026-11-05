# Router compatibility

EPF talks to a protocol, rather than maintaining a list of brand-specific passwords or web pages. A firmware update, guest network, VPN, ISP restriction, or disabled UPnP setting can change the result for the same router model.

## What changed

Previously, every operation required `GetGenericPortMappingEntry` to enumerate the router's table. EPF now asks about the exact port with `GetSpecificPortMappingEntry`, using the service advertised by the router. This covers WANIPConnection:1, WANIPConnection:2, and WANPPPConnection:1. The latter two service types were already discovered by `igd-next`; the new targeted inspection works with all three.

That removes full-table enumeration as a requirement for opening, refreshing, and closing EPF rules. A router can reject full-table listing and still work if it supports individual reads and finite leases. Fault 401 (unsupported action) falls back to the original listing path. Only fault 714 (no such entry) counts as a free port. Timeouts, permission errors, malformed data, and other faults stop the operation.

Discovery now retries SSDP directly to the default gateway if multicast discovery fails or selects another device. This can help where multicast is filtered but unicast SSDP is available. It cannot make a router implement UPnP or bypass a guest-network access policy.

## Documented router families

These are candidates based on public documentation, **not a hardware-certified compatibility list**. None of these physical routers was available for this change.

| Router family                                   | Public evidence                                                                                                                                                                                         | What EPF needs                                                                                                                                                       |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| OpenWrt with MiniUPnPd                          | [OpenWrt setup guide](https://openwrt.org/docs/guide-user/firewall/upnp/miniupnpd), [public package implementation](https://github.com/openwrt/packages/blob/master/net/miniupnpd/files/miniupnpd.init) | UPnP enabled on the LAN, an allowed port range, and permitted temporary leases. With secure mode enabled, use this computer as the destination.                      |
| TP-Link routers and modem routers with UPnP     | [TP-Link's UPnP guide](https://www.tp-link.com/in/support/faq/4624/)                                                                                                                                    | Enable UPnP if you choose to use it; the setting location varies by firmware. EPF must be on the main LAN and the router must allow inspection and temporary leases. |
| ASUS routers exposing UPnP                      | [ASUS's UPnP/NAT-PMP settings documentation](https://www.asus.com/us/support/faq/1045028/)                                                                                                              | UPnP must be enabled and its access policy must permit the request. This integration uses UPnP; NAT-PMP remains unimplemented.                                       |
| DSL/PPP gateways advertising WANPPPConnection:1 | [UPnP WANPPPConnection specification](https://upnp.org/specs/gw/UPnP-gw-WANPPPConnection-v1-Service.pdf)                                                                                                | The advertised PPP service must support mapping inspection, creation, deletion, and finite leases. No brand-specific SOAP namespace is hardcoded.                    |

Enabling UPnP is a router-owner decision. EPF doesn't change that setting, log in to the router, or store its password. Router discovery remains read-only.

## Research and implementation references

Reviewed on 2026-11-05:

- [UPnP WANIPConnection v1 service specification](https://upnp.org/specs/gw/UPnP-gw-WANIPConnection-v1-Service.pdf): specific mapping arguments and fault 714.
- [UPnP WANIPConnection v2 service specification](https://upnp.org/specs/gw/UPnP-gw-WANIPConnection-v2-Service.pdf): mapping keys and v2 service behavior.
- [MiniUPnP client source](https://github.com/miniupnp/miniupnp/blob/master/miniupnpc/src/upnpc.c): public examples of generic and specific mapping queries. EPF's new SOAP reader is an independent Rust implementation; no source was copied.
- [RFC 6886](https://www.rfc-editor.org/rfc/rfc6886.html), [RFC 6887](https://www.rfc-editor.org/rfc/rfc6887.html), and [crab_nat](https://github.com/ryco117/crab_nat): NAT-PMP and PCP were reviewed as future integrations. These protocols do not provide UPnP's description-based ownership inspection, so they need a separate lifecycle design rather than being presented as interchangeable UPnP adapters. They remain unimplemented.

## Verification and reporting

Tests exercise namespaced IP v1/v2 and PPP responses, decoded descriptions, fault classification, invalid and duplicate fields, actual loopback HTTP requests, key deduplication, and a rule lifecycle with full-table listing disabled. They also verify that a changed ownership description blocks deletion. Existing journal schema and router IDs are unchanged, so previously saved rules and profiles continue to load.

Hardware forwarding, lease expiry, and internet accessibility remain unverified for these families. To report a router result, include model, firmware, operating system, advertised UPnP service, EPF error message, and whether a finite mapping was created and closed. Redact public IPs and private network details before posting logs. Never include router passwords or your configuration journal.
