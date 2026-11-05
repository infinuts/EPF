# Contributing to EPF

Router behavior varies a lot. A clear report from a router we haven't tested can be as useful as a code contribution.

Include the EPF version, operating system, router manufacturer/model, and the exact error. Say whether you were on a VPN, a guest network, or behind a second router. Remove public IP addresses and other personal details before opening an issue.

## Find your way around

| Path                | Responsibility                                                    |
| ------------------- | ----------------------------------------------------------------- |
| `crates/epf-core`   | Discovery, UPnP adapter, rule ownership, persistence, diagnostics |
| `crates/epf-cli`    | Argument parsing, confirmation, terminal and JSON output          |
| `src-tauri`         | Native app shell and asynchronous IPC commands                    |
| `src`               | React interface and clearly separated browser preview             |
| `tests/e2e`         | Browser flows using disposable sample data                        |
| `.github/workflows` | Checks and draft release builds                                   |

Keep networking logic in the core crate. A new forwarding protocol should implement the `Router` trait, use the same confirmation boundary, and preserve ownership checks. Don't add router-password storage to work around an unsupported protocol.

Run the checks in the README before sending a pull request. Add tests when behavior at a boundary changes, especially confirmation, destination validation, partial operations, or persistence. A bug fix should explain what failed and how the test reproduces it.

## Compatibility

The local JSON schema starts at version 1. New optional fields need serde defaults, so existing files still load. A version this build doesn't understand must produce an error without rewriting the file. Changes that alter saved meaning need an explicit migration with fixtures and recovery instructions.

Keep existing CLI commands and flags working when adding features. Machine-readable field changes need a release note. Before version 1.0, deliberate breaking changes can happen, but they must be documented and must not discard users' saved rules.

Small commits should represent useful steps: core behavior, interface, regression test, or documentation. The git history is a way to inspect and revert work; it doesn't by itself make changes backward compatible.

## Release checklist

1. Update the version in the workspace Cargo manifest, package.json, and Tauri config. Regenerate lockfiles.
2. Run the check matrix and inspect the desktop on Windows, macOS, and Linux.
3. Test on an actual UPnP router with explicit permission to open a temporary port. Check both protocols, expiry, and close behavior.
4. Push a `vVERSION` tag or start the release workflow. It creates a draft prerelease.
5. Inspect the packages, signing status, and CLI archives. Record known platform limits before publishing.

Never publish a draft just because compilation passed. The native packages need installation and router testing too.
