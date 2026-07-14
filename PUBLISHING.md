# Publishing Process Guard 1.0.2

1. Create a new GitHub repository and upload the contents of the generated `repository` folder.
2. Commit and push the source files.
3. Create a GitHub release tagged `v1.0.2`.
4. Upload every file from the generated `release-assets` folder to that release.
5. Paste `RELEASE_NOTES_1.0.2.md` into the release notes.
6. Verify each uploaded file against `SHA256SUMS.txt`.

Do not upload local `ProcessGuard_Settings.txt`, Sentinel history, snapshots, reports, the Cargo `target` directory, or any Windows Credential Manager export.
