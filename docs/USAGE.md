# Process Guard Usage Guide

## Monitor and Manage Processes

![Process Guard main process monitor](images/process-guard-main.png)

1. Use **Monitor** for the live process list and system resource summary.
2. Search by process name, company, type, or description. Use the clear `X` to reset the search immediately.
3. Click a group to select it. Double-click it, use its arrow, or choose **View > Expand selected groups** to show individual PIDs.
4. Read the safety color and reason before taking action: green is considered safe, orange needs review, and red is blocked or high risk.
5. Right-click a selected group or PID for relevant actions such as details, file location, suspend, resume, priority, and end process.
6. Use **Select Safe** to mark visible safe items, then review the highlighted selection before ending anything.

The first Process/Group column stays visible while the remaining columns scroll horizontally. Safety labels are conservative guidance, not a guarantee; ending a process can still close an application or discard unsaved work.

## Configure Sentinel

Open **Help > Sentinel AI Settings**.

![Sentinel engine, model tier, and validation settings](images/sentinel-settings.png)

1. Choose a local CLI engine or one of the supported API providers.
2. Choose a suggested model or enter an exact model ID supported by your account.
3. Prefer **Included** or **Free Tier** choices when available. **Standard** and **Premium** models can require billing and may have higher usage costs.
4. For API providers, enter the key for the current Windows account. Keys are stored in Windows Credential Manager, not in Process Guard files.
5. Select **Test + Save + Restart**. Process Guard sends a tiny connectivity prompt to the exact engine and model.
6. On success, Process Guard saves the configuration and restarts. On failure, it stays open, shows the error, and leaves the previous settings and stored key unchanged.

Local CLI engines must already be installed and signed in. API mode is available only in the Setup-installed version. Provider requests can consume quota or incur a small charge, including the validation request.

## Use Sentinel Chat

Select one or more processes and open Sentinel from the process menu or toolbar. Follow-up questions stay in the current conversation. The history view can reopen, pin, unpin, regenerate, or delete saved conversations; pinned conversations are preserved when clearing unpinned history.

Drag the Sentinel title area to move the chat. Use the line-shaped `-` and `+` controls to resize it, or **Fit** to return it to a practical size. Detach the chat when it needs to move outside the main Process Guard window.

## Installation and Privacy

Use `ProcessGuard-Setup-1.0.2.exe` for API-provider support, Start menu shortcuts, the installed marker, and the uninstall wizard. The portable `ProcessGuard.exe` supports local CLI engines and stores its local history/settings beside the executable.

Before sending a request, Process Guard shows which engine will receive the process context. Do not send sensitive command lines or paths to a third-party provider unless its data and retention policies are acceptable for your use.
