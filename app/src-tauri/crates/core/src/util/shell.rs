use std::path::PathBuf;

/// Find a usable bash executable on the system
pub fn find_bash() -> Option<PathBuf> {
    // Try common Windows Git Bash locations first
    #[cfg(target_os = "windows")]
    {
        let candidates = [
            r"C:\Program Files\Git\bin\bash.exe",
            r"C:\Program Files (x86)\Git\bin\bash.exe",
            r"C:\msys64\usr\bin\bash.exe",
        ];
        for path in &candidates {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }
        // Try PATH lookup on Windows
        if let Ok(output) = std::process::Command::new("where").arg("bash").output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = stdout.lines().next() {
                    let p = PathBuf::from(line.trim());
                    if p.exists() {
                        return Some(p);
                    }
                }
            }
        }
    }

    // Unix: bash should be in PATH
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(output) = std::process::Command::new("which").arg("bash").output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let p = PathBuf::from(stdout.trim());
                if p.exists() {
                    return Some(p);
                }
            }
        }
        // Fallback
        let p = PathBuf::from("/bin/bash");
        if p.exists() {
            return Some(p);
        }
    }

    None
}
