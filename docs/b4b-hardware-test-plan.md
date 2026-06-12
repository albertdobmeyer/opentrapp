# B4b hardware test plan — verifying the daemon/viewer defer

**What this verifies:** Phase B slice B4b (ADR-0019) — the GUI deferring perimeter
ownership to the bundled headless `opentrapp-daemon`, and the resting-memory win
that follows. None of this can be checked in CI (no display, no container
runtime), so it is **unverified until this plan passes on capable hardware**
(CLAUDE.md §11).

- **Build under test:** any installer built from commit `f4ffac8` or later (the
  daemon ships as a tauri `externalBin` sidecar from B4b step 1).
- **Machine:** one that can run the full five-container perimeter without
  swap-storming (NOT the 7.2 GB reference laptop). Windows or Linux preferred for
  the process-inspection steps.
- **The opt-in:** the whole defer is gated behind the env var
  `OPENTRAPP_DAEMON_DEFER=1`. With it unset, the app behaves exactly as today
  (the GUI self-owns) — that is itself test 1.
- **Tools:** `opentrapp-daemon --status` (the bundled binary, next to the app
  executable), plus `smem`/`htop` (Linux) or Task Manager / `Get-Process`
  (Windows) for RSS.

For each test, record PASS/FAIL and the observed numbers. A single FAIL means the
defer must stay opt-in/off until fixed.

---

## Test 1 — Inert default (defer OFF)

**Why:** proves the shipped default is byte-identical to pre-B4b behavior.

1. Launch the app normally (do **not** set `OPENTRAPP_DAEMON_DEFER`).
2. Complete the wizard / reach the running state as usual.

**PASS when:** the app owns the perimeter exactly as before — `~/.opentrapp/runguard.pid`
holds the **GUI's** PID; `opentrapp-daemon --status` prints `runguard: free (no
live owner)` is **not** expected here (the GUI holds it, so it shows the GUI pid
as "another live owner" — that's fine, it just means *something* owns it). No
`[daemon-link]` lines appear in the app log. Closing the app tears the perimeter
down as today.

---

## Test 2 — Daemon launches and takes ownership (defer ON)

**Why:** the core handoff — the GUI launches the daemon detached and steps back.

1. Quit the app and confirm no perimeter containers remain (`podman ps`).
2. Launch with the flag set, e.g. (Linux) `OPENTRAPP_DAEMON_DEFER=1 ./OpenTrApp_*.AppImage`,
   or set the user/system env var and launch normally on Windows/macOS.
3. Watch the app log.

**PASS when:**
- The log shows `[daemon-link] daemon launched and owns the perimeter`.
- `opentrapp-daemon --status` reports `runguard: held by another live owner
  (pid=…)` where the pid is the **daemon**, not the GUI.
- `podman ps` shows the perimeter coming up (driven by the daemon, not the GUI).

**FAIL signs:** `[daemon-link] … — self-owning` (no bundled binary found / spawn
failed / daemon didn't take the guard). If so, capture the exact log line — it
names which fallback path was hit.

---

## Test 3 — The resting-memory win (the point of Phase B)

**Why:** the reason the daemon split exists — a lean viewer + a WebKit-free owner.

1. With defer ON and the perimeter up (test 2), open the dashboard.
2. Measure RSS of: the GUI process(es) (`opentrapp` + `WebKitWebProcess` +
   `WebKitNetworkProcess`) and the `opentrapp-daemon` process.
3. **Close the dashboard window.** Re-measure.
4. **Quit the GUI entirely** (tray Quit, or close the viewer process). Re-measure.

**PASS when:**
- The `opentrapp-daemon` process is **~30–60 MB** and has **no** WebKit child
  processes (`ldd $(which opentrapp-daemon)` shows no `libwebkit2gtk`).
- After **quitting the GUI**, the **daemon and the perimeter keep running** — this
  is the whole point. `opentrapp-daemon --status` still shows it owning the guard;
  `podman ps` still shows the perimeter. The GUI's ~220 MB (WebKit + GTK) is gone;
  only the ~30–60 MB daemon + the perimeter remain resident.
- Re-launching the GUI (defer ON) **reattaches as a viewer** (does not spawn a
  second owner — `[daemon-link]` reports "a daemon already owns it"), and the
  dashboard shows the live perimeter state.

**Record:** GUI RSS (open), GUI RSS (closed), daemon RSS, and total resident with
the GUI quit. The target is total-resting ≈ daemon (~30–60 MB) + perimeter, with
**zero** GUI/WebKit resident.

---

## Test 4 — Control flow through the daemon

**Why:** when deferred, GUI actions must route through the daemon, not fight it.

1. With defer ON, in the GUI use **Pause** → then **Resume** → then **Restart**.
2. After each, check `opentrapp-daemon --status` and `podman ps`.

**PASS when:**
- **Pause:** `--status` shows `paused=true`; containers stop. (User pause — it does
  **not** arm the idle waker and does **not** auto-wake on a message.)
- **Resume:** `paused`/`dormant` clear; the perimeter comes back up.
- **Restart:** the perimeter cycles down then up.
- These are driven by the daemon (the GUI only dropped a request file under
  `~/.opentrapp/control/`; confirm the file is consumed, i.e. the dir is empty
  after the daemon's next tick). Note: up to ~30 s latency is expected (the
  supervisor tick) — this is the durable-inbox tradeoff, not a bug.

---

## Test 5 — Idle auto-pause + wake (daemon-owned)

**Why:** the daemon, not the GUI, must run idle auto-pause when deferred.

1. With defer ON and a Telegram bot token configured, leave it idle ~12 min.
2. Then send a Telegram message.

**PASS when:**
- After the idle threshold, `--status` shows `dormant=true` and the perimeter
  stops (RAM drops to ~daemon-only). The GUI (if open) reflects "sleeping".
- The Telegram message **wakes** it: the perimeter resumes and the bot replies
  **exactly once** (no double-process, no lost message — ADR-0018).
- Confirm the GUI's own watchdog did **not** also try to pause (no duplicate
  pause/resume churn in the logs).

---

## Test 6 — Crash resilience (the ADR-0019 fragility fix)

**Why:** the security boundary must survive a GUI/webview crash.

1. With defer ON and the perimeter up, **kill the GUI's webview process**
   (`kill -BUS <WebKitWebProcess pid>` on Linux, or force-kill the GUI).

**PASS when:** the `opentrapp-daemon` and the perimeter are **unaffected** —
`--status` still shows it owning the guard, `podman ps` still shows the perimeter,
network isolation still holds. Re-launching the GUI reattaches cleanly.

---

## Test 7 — Fallback safety net

**Why:** any defer failure must degrade to today's self-owning, never a broken app.

1. Temporarily rename/remove the bundled `opentrapp-daemon` next to the app
   executable, then launch with `OPENTRAPP_DAEMON_DEFER=1`.

**PASS when:** the log shows `[daemon-link] … no bundled daemon found —
self-owning`, and the app proceeds to own the perimeter itself exactly as in
test 1. (Restore the binary afterward.)

---

## Reporting

If tests 1–7 pass, B4b can be promoted from opt-in to default (flip the
`OPENTRAPP_DAEMON_DEFER` default in `daemon_link::defer_enabled`, update ADR-0019
status, and record the measured resting RSS in
`docs/footprint-and-device-usability.md` alongside the Phase A §10.4 numbers).
Until then it stays opt-in, and the resting-memory claim is **scoped to "verified
on capable hardware"**, not asserted for the shipped default.
