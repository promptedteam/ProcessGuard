# Security Policy

## Supported Version

Security fixes are currently applied to the latest published Process Guard release.

## Reporting a Vulnerability

Use a private GitHub security advisory when the repository enables that feature. Do not place API keys, local process reports, command lines, usernames, or private executable paths in a public issue.

Include the Process Guard version, Windows version, reproduction steps, expected behavior, and observed impact. Remove secrets and personal process data from screenshots and logs.

## Credential Design

Installed API keys are stored in Windows Credential Manager under targets beginning with `ProcessGuard/Sentinel/`. The portable build cannot configure or read these credentials. Full-data uninstall removes them; a preserve-data uninstall intentionally keeps them for reinstall.
