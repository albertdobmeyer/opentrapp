// OpenCli-Container: Global Proxy Bootstrap
// Forces all undici/fetch requests through the vault proxy BEFORE OpenClaw starts.
// This fixes grammY (Telegram) not respecting HTTP_PROXY env vars.

import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);

// Use undici from OpenClaw's dependencies (not installed globally)
const undiciPath = '/usr/local/lib/node_modules/openclaw/node_modules/undici';
const { setGlobalDispatcher, ProxyAgent } = require(undiciPath);

const proxyUrl = process.env.HTTPS_PROXY || process.env.HTTP_PROXY;
if (proxyUrl) {
  const agent = new ProxyAgent(proxyUrl);
  setGlobalDispatcher(agent);
  console.log(`[vault] Global proxy dispatcher set: ${proxyUrl}`);
}
