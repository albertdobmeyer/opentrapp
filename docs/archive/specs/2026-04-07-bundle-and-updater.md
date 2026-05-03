# Spec: Cross-Platform Bundle & Updater Configuration

**Date:** 2026-04-07
**Phase:** H (Finalization Roadmap v4)
**Depends on:** Nothing
**Blocks:** Phase J (release — can't ship without installable binaries)

---

## Problem

CI already builds for 4 platform targets (Linux x64, macOS ARM, macOS Intel, Windows x64) via `tauri-apps/tauri-action@v0`. However:

1. **`tauri.conf.json` only has `bundle.windows`** — macOS and Linux sections are missing, so platform-specific metadata (dmg settings, deb dependencies, desktop integration) uses Tauri defaults.
2. **Updater is inactive** — `plugins.updater.active: false` and `pubkey: ""`. The plugin is installed (`tauri-plugin-updater` in both Cargo.toml and package.json), CI already sets `includeUpdaterJson: true` and references `TAURI_SIGNING_PRIVATE_KEY`, but nothing is configured.
3. **No signing ceremony done** — No keypair exists for update signing.

---

## Current State

File: `app/src-tauri/tauri.conf.json`

```json
"bundle": {
  "active": true,
  "category": "DeveloperTool",
  "shortDescription": "Security-first desktop GUI for the OpenClaw ecosystem",
  "copyright": "Copyright (c) 2026 Albert Dobmeyer",
  "icon": ["icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png", "icons/icon.icns", "icons/icon.ico"],
  "windows": {
    "certificateThumbprint": null,
    "digestAlgorithm": "sha256",
    "timestampUrl": "http://timestamp.sectigo.com"
  }
}
```

```json
"plugins": {
  "updater": {
    "active": false,
    "endpoints": [
      "https://github.com/albertdobmeyer/lobster-trapp/releases/latest/download/latest.json"
    ],
    "pubkey": ""
  }
}
```

CI references (`.github/workflows/ci.yml:166-168`):
```yaml
TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
```

---

## Design

### H1: macOS Bundle Config

Add `bundle.macOS` section to `tauri.conf.json`:

```json
"macOS": {
  "minimumSystemVersion": "10.15",
  "dmg": {
    "appPosition": { "x": 180, "y": 170 },
    "applicationFolderPosition": { "x": 480, "y": 170 },
    "windowSize": { "width": 660, "height": 400 }
  },
  "signingIdentity": null,
  "providerShortName": null
}
```

**Notes:**
- `minimumSystemVersion: "10.15"` (Catalina) — WebKit2 requirement for Tauri 2
- `signingIdentity: null` — builds unsigned. For signed builds, set to Developer ID Application certificate name and configure via `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY` env vars in CI
- Notarization requires `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` — deferred until Apple Developer account is set up

### H2: Linux Bundle Config

Add `bundle.linux` section to `tauri.conf.json`:

```json
"linux": {
  "deb": {
    "depends": ["libwebkit2gtk-4.1-0", "libappindicator3-1"],
    "section": "utils",
    "priority": "optional"
  },
  "appimage": {
    "bundleMediaFramework": false
  }
}
```

**Notes:**
- Default targets include both deb and AppImage — Tauri 2 builds both automatically
- `libwebkit2gtk-4.1-0` is the runtime WebKit dependency on Debian/Ubuntu
- `libappindicator3-1` provides system tray support
- AppImage is self-contained (no system deps) — primary distribution format for non-Debian

**Desktop file:** Tauri auto-generates from `tauri.conf.json` fields:
- Name: `productName` ("Lobster-TrApp")
- Comment: `shortDescription`
- Categories: `category` ("DeveloperTool" maps to `Development`)
- Icon: from `bundle.icon`

### H3: Updater Configuration

**Step 1: Generate signing keypair**

```bash
npx tauri signer generate -w ~/.tauri/lobster-trapp.key
```

This produces:
- `~/.tauri/lobster-trapp.key` (private key — NEVER commit)
- Public key string (stdout) — goes into `tauri.conf.json`

**Step 2: Store private key in GitHub**

```
Repository → Settings → Secrets and variables → Actions
  TAURI_SIGNING_PRIVATE_KEY = <contents of ~/.tauri/lobster-trapp.key>
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD = <password from generate step>
```

**Step 3: Update `tauri.conf.json`**

```json
"updater": {
  "active": true,
  "endpoints": [
    "https://github.com/albertdobmeyer/lobster-trapp/releases/latest/download/latest.json"
  ],
  "pubkey": "<public key from generate step>"
}
```

**Step 4: Verify CI produces `latest.json`**

The CI already has `includeUpdaterJson: true` in the `tauri-apps/tauri-action` config (line 177). When a tagged release is created, this generates a `latest.json` alongside the binaries.

### H4: Test Release Workflow

1. Push tag `v0.1.0-rc.1`
2. CI triggers `build-and-release` job for all 4 platforms
3. Verify draft release contains:
   - `lobster-trapp_0.1.0_amd64.deb` (Linux deb)
   - `lobster-trapp_0.1.0_amd64.AppImage` (Linux AppImage)
   - `Lobster-TrApp_0.1.0_aarch64.dmg` (macOS ARM)
   - `Lobster-TrApp_0.1.0_x64.dmg` (macOS Intel)
   - `Lobster-TrApp_0.1.0_x64-setup.exe` (Windows NSIS)
   - `latest.json` (updater manifest)
4. Download and install on at least one platform to verify the binary works

---

## Files to Modify

| Action | Path | Change |
|--------|------|--------|
| **Modify** | `app/src-tauri/tauri.conf.json` | Add `bundle.macOS`, `bundle.linux` sections; set `updater.active: true` and `updater.pubkey` |

---

## Deferred

- **macOS code signing + notarization:** Requires Apple Developer Program enrollment ($99/year). Unsigned builds work but show Gatekeeper warning. Can be added later by setting `signingIdentity` and CI env vars.
- **Windows code signing:** Current config supports it (`certificateThumbprint`). Requires EV certificate (~$200-400/year). Unsigned builds show SmartScreen warning.
- **Linux Flatpak/Snap:** Additional distribution channels. Not needed for initial release.

---

## Verification

1. `tauri.conf.json` passes JSON Schema validation
2. `cargo check` succeeds with new config
3. Local `npm run tauri build` produces a platform-appropriate installer
4. RC tag triggers CI release with all expected artifacts
5. Downloaded artifact installs and runs on target OS
