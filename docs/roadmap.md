# Roadmap

The first preview focuses on a shared engine, UPnP rules, and an interface that explains what happened. Broader router support comes after the lifecycle and packaging are reliable.

## Before the first public release

- Install and test the native packages on Windows, macOS, and Linux.
- Record results from physical routers, including manufacturer, model, firmware, lease behavior, and whether they preserve descriptions.
- Check accessibility with screen readers and keyboard-only use.
- Add release signing and macOS notarization when certificates are available.

## Next integrations

- NAT-PMP and PCP adapters with protocol-specific ownership and lifetime handling.
- A user-approved external reachability service with an explicit privacy contract and separate TCP/UDP behavior.
- Interface selection for multi-homed computers and VPN setups.
- Import/export for profiles with validation and schema migration fixtures.
- Explicit lease renewal and clearer expiry information based on router-reported leases.
- IPv6 firewall support where the router exposes it.

## Later

An optional npm wrapper could distribute the native CLI for developer convenience. It must remain optional; using the desktop app or standalone CLI should never require a JavaScript runtime.
