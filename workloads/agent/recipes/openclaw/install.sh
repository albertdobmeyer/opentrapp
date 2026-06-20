#!/bin/sh
# Agent recipe: OpenClaw — build-time install (runs in the Containerfile builder stage).
#
# Installs the OpenClaw runtime globally into /usr/local/lib/node_modules and slims it;
# the production stage COPYs node_modules from the builder. This is the OpenClaw recipe's
# build half — behavior-identical to the inline install it replaced (see git history of
# workloads/agent/Containerfile and recipes/README.md for the base/recipe contract).
set -eu

# git is required by some OpenClaw npm deps (build-only; not in the production image).
apk --no-cache add git

# OpenClaw requires Node >=22.12.0 (base image is node:22-alpine). --ignore-scripts skips
# node-llama-cpp's native build — the vault uses the proxy, not local LLMs. Pinned 2026.2.26
# (includes the Telegram-proxy-preservation fix, PR #30367).
npm install -g openclaw@2026.2.26 --ignore-scripts

# Slim: delete ONLY files Node and the TS transpiler never read at runtime — TypeScript
# DECLARATIONS (*.d.ts), sourcemaps (*.map), flow types (*.flow) — plus compile-time
# @types / bun-types. *.ts and *.md ARE runtime assets for OpenClaw (extensions load as TS;
# markdown workspace templates are read on the reply path) and MUST be kept. Cuts
# node_modules ~606MB -> ~406MB. See docs/specs/2026-06-06-image-conservative-prune.md.
cd /usr/local/lib/node_modules/openclaw
find . -type f \( -iname '*.d.ts' -o -iname '*.map' -o -iname '*.flow' \) -delete
rm -rf node_modules/@types node_modules/bun-types
