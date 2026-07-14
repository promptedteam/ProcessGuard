# Contributing

Process Guard is a native Windows Rust application. Keep changes focused, small, and consistent with the existing Win32/GDI implementation.

## Development

1. Install Rust stable with the MSVC target and Windows build tools.
2. Run `cargo fmt --check`.
3. Run `cargo test`.
4. Run `cargo build --release`.
5. Test process selection, grouped expansion, Sentinel history, settings persistence, tray behavior, and normal/admin relaunching on Windows.

Installer changes also require Inno Setup 6 and a test install/uninstall in a disposable per-user directory.

## Pull Requests

- Explain the behavior being changed and the user-visible impact.
- Add focused tests where logic can be isolated.
- Do not commit local reports, Sentinel history, settings, snapshots, API keys, `target`, or generated release folders.
- Do not weaken the blocked-process rules or installed-only API credential gate without a documented security reason.
- Keep AI provider failures free of raw headers and secrets.
