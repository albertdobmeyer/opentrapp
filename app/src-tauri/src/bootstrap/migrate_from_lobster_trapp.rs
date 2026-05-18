//! One-shot first-run migration from the legacy Lobster-TrApp install
//! (binary name `lobster-trapp`, bundle id `dev.lobster-trapp.app`,
//! state dirs `~/.lobster-trapp/` + `~/lobster-trapp/`) to OpenTrApp.
//!
//! Runs synchronously before `PerimeterStateStore` reads any marker
//! files so the store sees the new paths. Idempotent: writes a
//! `migrated-from-lobster-trapp` breadcrumb after success and no-ops on
//! subsequent launches.
//!
//! The migration preserves user data: paused/activated markers,
//! `.env` (API keys + bot token), container/volume names. Anything
//! that fails individually is logged and skipped — the user can finish
//! by hand from the toast emitted on completion.

use std::fs;
use std::path::PathBuf;
use std::process::Command as StdCommand;

/// Run the migration. Safe to call on every launch — short-circuits on
/// the breadcrumb file.
pub fn migrate_if_legacy_install() {
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return,
    };

    let new_dir = home.join(".opentrapp");
    let breadcrumb = new_dir.join("migrated-from-lobster-trapp");
    if breadcrumb.exists() {
        return;
    }

    let legacy_state_dir = home.join(".lobster-trapp");
    let legacy_env_dir = home.join("lobster-trapp");
    let new_env_dir = home.join("opentrapp");

    let mut anything_migrated = false;

    // State dir: ~/.lobster-trapp/ → ~/.opentrapp/. Move files
    // individually so a partially-populated ~/.opentrapp/ doesn't
    // block the migration (e.g. user already launched OpenTrApp once
    // and wrote a paused marker, then upgraded a second machine).
    if legacy_state_dir.is_dir() {
        let _ = fs::create_dir_all(&new_dir);
        if let Ok(entries) = fs::read_dir(&legacy_state_dir) {
            for entry in entries.flatten() {
                let src = entry.path();
                let dst = new_dir.join(entry.file_name());
                if dst.exists() {
                    eprintln!(
                        "[migrate] skip (already exists in new location): {}",
                        dst.display()
                    );
                    continue;
                }
                if let Err(e) = fs::rename(&src, &dst) {
                    eprintln!(
                        "[migrate] could not move {} → {}: {e}",
                        src.display(),
                        dst.display()
                    );
                } else {
                    anything_migrated = true;
                }
            }
        }
        // Best-effort cleanup of the now-empty legacy state dir.
        let _ = fs::remove_dir(&legacy_state_dir);
    }

    // Env dir: ~/lobster-trapp/ → ~/opentrapp/. Only the `.env` file
    // is documented in the user-facing path; move the whole directory
    // if it has nothing in the new location yet.
    if legacy_env_dir.is_dir() && !new_env_dir.exists() {
        if let Err(e) = fs::rename(&legacy_env_dir, &new_env_dir) {
            eprintln!(
                "[migrate] could not move env dir {} → {}: {e}",
                legacy_env_dir.display(),
                new_env_dir.display()
            );
        } else {
            anything_migrated = true;
        }
    }

    // Tauri settings dir: ~/.config/dev.lobster-trapp.app/ → ~/.config/com.opentrapp.app/
    let config_root = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home.join(".config"));
    let legacy_settings = config_root.join("dev.lobster-trapp.app");
    let new_settings = config_root.join("com.opentrapp.app");
    if legacy_settings.is_dir() && !new_settings.exists() {
        if let Err(e) = fs::rename(&legacy_settings, &new_settings) {
            eprintln!(
                "[migrate] could not move Tauri settings dir {} → {}: {e}",
                legacy_settings.display(),
                new_settings.display()
            );
        } else {
            anything_migrated = true;
        }
    }

    // Podman containers + volumes: anything created by the previous
    // compose project (`lobster-trapp_*`) is renamed to the new prefix
    // (`opentrapp_*`). compose itself doesn't care about names — it
    // labels by service — so this is purely cosmetic + lets future
    // `podman ps` greps work.
    if anything_migrated {
        rename_podman_objects("container", "rename");
        rename_podman_objects("volume", "rename");
    }

    // Write the breadcrumb regardless of whether anything moved — a
    // clean install will skip the migration on the next launch.
    let _ = fs::create_dir_all(&new_dir);
    if let Err(e) = fs::write(&breadcrumb, b"OpenTrApp migrated from Lobster-TrApp\n") {
        eprintln!("[migrate] could not write breadcrumb {}: {e}", breadcrumb.display());
    }

    if anything_migrated {
        eprintln!("[migrate] migrated legacy Lobster-TrApp state to OpenTrApp paths");
    }
}

/// Iterate every podman object of `kind` whose name starts with
/// `lobster-trapp_`, then call `podman <kind> <verb> <old> <new>` to
/// rename to the `opentrapp_` prefix. Best-effort — podman absence,
/// running containers, and rootless quirks all silently no-op.
fn rename_podman_objects(kind: &str, verb: &str) {
    let list = match StdCommand::new("podman")
        .args([kind, "ls", "--format", "{{.Names}}"])
        .output()
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
        _ => return,
    };

    for line in list.lines() {
        let name = line.trim();
        if let Some(stripped) = name.strip_prefix("lobster-trapp_") {
            let new_name = format!("opentrapp_{stripped}");
            let _ = StdCommand::new("podman")
                .args([kind, verb, name, &new_name])
                .status();
        }
    }
}
