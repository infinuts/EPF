# Reporting a security issue

Please avoid posting an exploit or private network information in a public issue. Use the repository's private vulnerability reporting feature when available: open the Security tab on [infinuts/EPF](https://github.com/infinuts/EPF), then choose **Report a vulnerability**. If private reporting isn't enabled, open an issue asking for a private contact without including exploit details.

Include the EPF version, operating system, the affected boundary, and enough detail to reproduce the problem on a test network. Don't include router passwords or public IP addresses.

The current preview supports only the code on the default branch. Published versions will list their support status here once releases exist.

EPF doesn't need administrator privileges to ask a UPnP router for a mapping. Its protection is explicit consent, input validation, local-only IPC, and ownership checks. The [architecture notes](docs/architecture.md) describe UPnP's concurrency and trust limits.
