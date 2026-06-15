# opentrapp-core

The headless, **Tauri-free** orchestration core of
[OpenTrApp](https://github.com/albertdobmeyer/opentrapp) — a registry-installable
CLI/daemon that runs an autonomous CLI agent inside a five-container security
perimeter on the user's own machine.

This crate is the reusable half of the application: the manifest / perimeter-state
contract, component discovery, the durable control channel, the idle supervisor,
and the boundary self-test embedding — with **no GUI dependencies**. It is what the
headless `opentrapp-daemon` links, and what alternative viewers (CLI, web, MCP) can
build on. By design it MUST NOT depend on `tauri` / `wry` / `webkit` (CI asserts the
daemon's dependency graph is WebKit-free).

Its data files (`perimeter.yml`, `boundary-selftest.sh`) are **vendored copies**
under `src/embedded/`, kept byte-identical to the canonical `resources/` + `tests/`
files by a drift-check, so the crate packages standalone.

See the architecture decision records for context:

- [ADR-0019 — headless daemon + GUI viewer split](https://github.com/albertdobmeyer/opentrapp/blob/main/docs/adr/0019-headless-daemon-gui-viewer-split.md)
- [ADR-0020 — product identity & distribution](https://github.com/albertdobmeyer/opentrapp/blob/main/docs/adr/0020-product-identity-and-distribution.md)

## Status

Pre-1.0 and evolving alongside OpenTrApp; the public API is not yet stable. Published
so the orchestration core is inspectable, reusable, and installable as part of the
project's OS-agnostic, vendor-neutral distribution
([ADR-0023](https://github.com/albertdobmeyer/opentrapp/blob/main/docs/adr/0023-distribution-and-packaging.md)).

## License

MIT.
