# MVP verification

Checked on 2026-11-05 in Windows x64 with Node.js 24.18.0 and Rust 1.99.1.

| Check                                                    | Result                                           |
| -------------------------------------------------------- | ------------------------------------------------ |
| Core Rust tests                                          | 12 passed                                        |
| CLI executable integration tests                         | 3 passed                                         |
| Frontend validation tests                                | 3 passed                                         |
| Playwright browser flows                                 | 6 passed                                         |
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

## Interface polish checks

The polished interface was visually checked at 1280 × 900 and 390 × 844, including the confirmation dialog and help page. Larger labels and darker secondary text replace the original small, pale text. Page transitions use scoped GSAP contexts and clean up when navigation changes. Reduced-motion settings disable both page transitions and help-page scroll effects.

The sixth browser test checks that the pinned help panel stays above the terminal section, reduced-motion content remains fully visible, and navigating away removes the pin wrapper. Existing confirmation, profile, diagnostics, unsupported-router, and narrow-layout flows also pass.

The Windows executable and NSIS installer were rebuilt after this polish. Repeating the native smoke-check launch was blocked by the execution approval policy, so the native runtime results above describe the earlier MVP build. The polished build has production compilation and browser-flow verification, but has not been runtime-verified through native IPC again.

To refresh the README screenshots, start `npm run dev` and run `node scripts/capture-preview.mjs`. This uses only the labeled browser preview and stops at confirmation without opening a port. Additional narrow-layout and help screenshots are saved in the ignored `artifacts/` directory.
