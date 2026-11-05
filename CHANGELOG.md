# Changelog

## 0.1.0 (unreleased)

- Targeted UPnP mapping reads for routers without working table enumeration, with a legacy fallback on explicit unsupported-action responses.
- Directed SSDP retry to the default gateway when multicast fails or discovers a different device.
- Sourced router compatibility guide and local HTTP protocol tests for IP v1/v2 and PPP services.

- Shared Rust engine for desktop and CLI.
- Local IPv4, default gateway, and UPnP discovery with optional router metadata.
- Confirmed TCP and UDP forwarding, custom destinations and ports, and finite leases.
- Local operation journal with ownership checks, locking, and atomic saves.
- Rule refresh, close and reopen flows, plus reusable profiles.
- CGNAT and upstream NAT indications with optional public IP lookup.
- React desktop interface with a separate labeled browser preview.
- Larger, higher-contrast interface text and clearer port confirmation details.
- Scoped GSAP page transitions and help-page motion that respects reduced-motion settings.
- Unit, CLI, and browser tests, plus cross-platform CI and draft release packaging.

NAT-PMP, PCP, IPv6 pinholes, external reachability testing, signing, and notarization are not included in this preview.
