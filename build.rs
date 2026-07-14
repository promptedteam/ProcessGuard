#[cfg(windows)]
fn main() {
    println!("cargo:rerun-if-changed=ProcessGuard.ico");

    let mut resource = winres::WindowsResource::new();
    resource
        .set_icon("ProcessGuard.ico")
        .set("FileDescription", "Process Guard")
        .set("ProductName", "Process Guard")
        .set("FileVersion", "1.0.2.0")
        .set("ProductVersion", "1.0.2.0")
        .set("OriginalFilename", "ProcessGuard.exe")
        .set("CompanyName", "Process Guard");

    if let Some(path) = windows_sdk_rc_dir() {
        resource.set_toolkit_path(&path);
    }

    if let Err(error) = resource.compile() {
        println!("cargo:warning=ProcessGuard icon resource was not embedded: {error}");
    }
}

#[cfg(not(windows))]
fn main() {}

#[cfg(windows)]
fn windows_sdk_rc_dir() -> Option<String> {
    let roots = [
        r"C:\Program Files (x86)\Windows Kits\10\bin",
        r"C:\Program Files\Windows Kits\10\bin",
        r"C:\Program Files (x86)\Windows Kits\8.1\bin",
    ];

    for root in roots {
        let root = std::path::Path::new(root);
        if !root.exists() {
            continue;
        }

        let mut candidates = Vec::new();
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                let path = entry.path().join("x64").join("rc.exe");
                if path.exists() {
                    candidates.push(path);
                }
            }
        }

        candidates.sort();
        if let Some(path) = candidates
            .pop()
            .and_then(|path| path.parent().map(|p| p.to_path_buf()))
        {
            return Some(path.display().to_string());
        }
    }

    None
}
