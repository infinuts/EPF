# MVP verification

Checked on 2026-11-05 in Windows x64 with Node.js 24.18.0 and Rust 1.99.1.

| Check                                                    | Result                                           |
| -------------------------------------------------------- | ------------------------------------------------ |
| Core Rust tests                                          | 12 passed                                        |
| CLI executable integration tests                         | 3 passed                                         |
| Frontend validation tests                                | 3 passed                                         |
| Playwright browser flows                                 | 5 passed                                         |
| TypeScript, ESLint, version consistency                  | Passed                                           |
| Rust formatting and core/CLI Clippy with warnings denied | Passed                                           |
| Production frontend bundle                               | Built                                            |
| npm dependency audit                                     | No reported vulnerabilities at verification time |
| Native Tauri desktop compile                             | Passed                                           |
| Windows release executable                               | Built and launched                               |
| Windows NSIS installer                                   | Built, approximately 3 MB                        |
| Windows standalone CLI archive                           | Built, approximately 2 MB                        |

The bundled desktop executable was launched independently of the development server. A WebView2 smoke check confirmed that the app used native data rather than the browser preview, completed a diagnostics request through Tauri IPC and the Rust engine, and rejected `open_ports` when confirmation was false. No browser exceptions or native stderr output appeared during the smoke check. Remote debugging was used only for this test, and the test app process was stopped afterward.

The real CLI detected the default IPv4 interface and gateway. No UPnP router responded on the available network. No real port was opened or closed, and public IP lookup was left disabled.

## What this does not establish

- Actual forwarding, deletion, expiry, or internet reachability on a physical router.
- Installation and uninstallation behavior of the NSIS installer. The bundled executable was exercised directly.
- Runtime behavior or installer quality on Linux or macOS. Build and check workflows are provided but were not executed on those hosts in this session.
- Code signing, notarization, or an external reachability service. The Windows preview installer is unsigned.

The local Windows build used GNU Rust with GCC from WinLibs because MSVC Build Tools were absent. An earlier LLVM-MinGW combination crashed while running Tauri's build script; the standard GCC toolchain resolved it. GNU linking emitted a multiple-manifest warning, so the usual MSVC release path still needs CI and installation verification.

Local build outputs and SHA-256 checksums are in `artifacts/`, which is ignored by Git. The release workflow targets the normal MSVC Windows toolchain and creates draft prereleases for maintainer review.

## Repeat the native smoke check

Launch a test copy of the Windows app with the process environment variable `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9227`, then run:

```sh
node scripts/smoke-native.mjs
```

The script reads network state and tests a rejected, unconfirmed request. It never approves a router mutation or enables public IP lookup. Stop the test app after checking it; remote debugging is not part of normal EPF startup.
