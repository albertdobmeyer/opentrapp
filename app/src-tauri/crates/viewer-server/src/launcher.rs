//! On-demand viewer launcher (ADR-0022 §2.3 / step 4): open the user's default browser to the bare
//! loopback URL with the single-use launch nonce in the URL *fragment* (`#n=…`). Per-OS opener:
//! `xdg-open` (Linux), `open` (macOS), `start` (Windows). Deliberately NOT `tauri-plugin-shell` —
//! the whole point of the viewer is to carry no Tauri/GTK surface.
//!
//! Security note: the nonce rides the opener's argv briefly. That is acceptable — it is single-use
//! and short-TTL (burned on the first `/api/session` exchange, §2.3), it never reaches a server in a
//! request line (URL fragments are not sent), and the opener is a same-user local process. The
//! long-lived bearer is NEVER passed here.

use std::io;
use std::process::Command;

/// Build the OS-appropriate "open this URL in the default browser" command. Pure (no spawn) so the
/// argument wiring is unit-testable without actually launching a browser.
fn browser_open_command(url: &str) -> Command {
    #[cfg(target_os = "linux")]
    {
        let mut c = Command::new("xdg-open");
        c.arg(url);
        c
    }
    #[cfg(target_os = "macos")]
    {
        let mut c = Command::new("open");
        c.arg(url);
        c
    }
    #[cfg(target_os = "windows")]
    {
        // `start` is a `cmd` builtin; the empty "" is its required window-title argument, so the URL
        // is treated as the target rather than the title.
        let mut c = Command::new("cmd");
        c.args(["/c", "start", "", url]);
        c
    }
}

/// Open `url` in the user's default browser, detached — we spawn the opener and do not wait on it
/// (it returns immediately once the browser is handed the URL).
pub fn open_browser(url: &str) -> io::Result<()> {
    browser_open_command(url).spawn().map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn linux_opens_via_xdg_open_with_exactly_the_url() {
        let url = "http://127.0.0.1:8080/#n=deadbeefcafebabe";
        let cmd = browser_open_command(url);
        assert_eq!(cmd.get_program().to_str(), Some("xdg-open"));
        let args: Vec<&str> = cmd.get_args().map(|a| a.to_str().unwrap()).collect();
        assert_eq!(args, [url], "the nonce URL is the single argument — no extra flags");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn macos_opens_via_open_with_exactly_the_url() {
        let url = "http://127.0.0.1:8080/#n=abc";
        let cmd = browser_open_command(url);
        assert_eq!(cmd.get_program().to_str(), Some("open"));
        let args: Vec<&str> = cmd.get_args().map(|a| a.to_str().unwrap()).collect();
        assert_eq!(args, [url]);
    }
}
