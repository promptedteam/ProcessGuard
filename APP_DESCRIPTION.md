# Process Guard Description

Process Guard is a compact, open-source Windows process manager built entirely with native Rust and Win32. It presents every running process in clear app groups, exposes live CPU, memory, disk, network, thread, and handle activity, and applies conservative green, orange, and red safety guidance before users take action.

The application includes multi-selection, safe grouped termination, live monitoring reports, process trees, connection inspection, signature checks, snapshots, watchlists, resource alerts, automatic efficiency rules, process launch controls, system tray operation, and normal/admin relaunching. Its Sentinel chat explains selected processes, preserves searchable conversation history, and can use Codex CLI, Claude CLI, Gemini CLI, or ten user-configured API providers. Suggested models are labeled by access/cost tier, and provider/model changes are tested before settings or staged credentials are saved.

Process Guard has no embedded browser runtime and does not bundle an AI model. API mode is available only after installation; keys are stored for the current Windows account in Windows Credential Manager and are never included in app settings, source code, binaries, or release archives. Safety classifications remain guidance, and users should review process identity, publisher, path, and impact before ending anything.
