import { invoke } from "@tauri-apps/api/core";

import {
  listComponents,
  getComponent,
  setMonorepoRoot,
  runCommand,
  loadOptions,
  startStream,
  stopStream,
  readConfig,
  writeConfig,
  runHealthProbe,
  getStatus,
  checkPrerequisites,
  initSubmodules,
  createConfigFromTemplate,
  // security-critical + lifecycle wrappers
  saveCredentials,
  readRuntimeEnv,
  validateAnthropicKey,
  deriveTelegramBotUrl,
  applyAllowlistDecision,
  listEgressApprovals,
  restartPerimeter,
  pausePerimeter,
  resumePerimeter,
  getPerimeterState,
  listWorkflows,
  executeWorkflow,
  telegramDeleteWebhook,
  telegramPollForStart,
  telegramSendMessage,
  telegramAdvanceOffset,
  getSentinelActivity,
  sentinelJudge,
} from "./tauri";

const mockInvoke = vi.mocked(invoke);

beforeEach(() => {
  mockInvoke.mockReset();
});

describe("IPC contract: each function calls invoke with correct command and args", () => {
  test("listComponents calls list_components", async () => {
    mockInvoke.mockResolvedValue([]);
    await listComponents();
    expect(mockInvoke).toHaveBeenCalledWith("list_components");
  });

  test("setMonorepoRoot calls set_monorepo_root with path", async () => {
    mockInvoke.mockResolvedValue([]);
    await setMonorepoRoot("/some/path");
    expect(mockInvoke).toHaveBeenCalledWith("set_monorepo_root", {
      path: "/some/path",
    });
  });

  test("getComponent calls get_component with componentId", async () => {
    mockInvoke.mockResolvedValue({});
    await getComponent("agent");
    expect(mockInvoke).toHaveBeenCalledWith("get_component", {
      componentId: "agent",
    });
  });

  test("runCommand calls run_command with componentId, commandId, args", async () => {
    mockInvoke.mockResolvedValue({});
    await runCommand("agent", "start", { env: "prod" });
    expect(mockInvoke).toHaveBeenCalledWith("run_command", {
      componentId: "agent",
      commandId: "start",
      args: { env: "prod" },
    });
  });

  test("runCommand defaults args to empty object", async () => {
    mockInvoke.mockResolvedValue({});
    await runCommand("agent", "start");
    expect(mockInvoke).toHaveBeenCalledWith("run_command", {
      componentId: "agent",
      commandId: "start",
      args: {},
    });
  });

  test("loadOptions calls load_options with timeout", async () => {
    mockInvoke.mockResolvedValue([]);
    await loadOptions("agent", "docker ps", 10);
    expect(mockInvoke).toHaveBeenCalledWith("load_options", {
      componentId: "agent",
      commandString: "docker ps",
      timeoutSeconds: 10,
    });
  });

  test("loadOptions defaults timeout to 5", async () => {
    mockInvoke.mockResolvedValue([]);
    await loadOptions("agent", "docker ps");
    expect(mockInvoke).toHaveBeenCalledWith("load_options", {
      componentId: "agent",
      commandString: "docker ps",
      timeoutSeconds: 5,
    });
  });

  test("startStream calls start_stream", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await startStream("agent", "logs", { tail: "100" });
    expect(mockInvoke).toHaveBeenCalledWith("start_stream", {
      componentId: "agent",
      commandId: "logs",
      args: { tail: "100" },
    });
  });

  test("stopStream calls stop_stream", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await stopStream("agent", "logs");
    expect(mockInvoke).toHaveBeenCalledWith("stop_stream", {
      componentId: "agent",
      commandId: "logs",
    });
  });

  test("readConfig calls read_config", async () => {
    mockInvoke.mockResolvedValue("content");
    await readConfig("agent", "config.yml");
    expect(mockInvoke).toHaveBeenCalledWith("read_config", {
      componentId: "agent",
      configPath: "config.yml",
    });
  });

  test("writeConfig calls write_config", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await writeConfig("agent", "config.yml", "new content");
    expect(mockInvoke).toHaveBeenCalledWith("write_config", {
      componentId: "agent",
      configPath: "config.yml",
      content: "new content",
    });
  });

  test("runHealthProbe calls run_health_probe with default timeout", async () => {
    mockInvoke.mockResolvedValue({});
    await runHealthProbe("agent", "docker ps");
    expect(mockInvoke).toHaveBeenCalledWith("run_health_probe", {
      componentId: "agent",
      probeCommand: "docker ps",
      timeoutSeconds: 10,
    });
  });

  test("getStatus calls get_status", async () => {
    mockInvoke.mockResolvedValue({});
    await getStatus("agent");
    expect(mockInvoke).toHaveBeenCalledWith("get_status", {
      componentId: "agent",
    });
  });

  test("checkPrerequisites calls check_prerequisites", async () => {
    mockInvoke.mockResolvedValue({});
    await checkPrerequisites();
    expect(mockInvoke).toHaveBeenCalledWith("check_prerequisites");
  });

  test("initSubmodules calls init_submodules", async () => {
    mockInvoke.mockResolvedValue("ok");
    await initSubmodules();
    expect(mockInvoke).toHaveBeenCalledWith("init_submodules");
  });

  test("createConfigFromTemplate calls with correct args", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await createConfigFromTemplate("vault", ".env", ".env.example");
    expect(mockInvoke).toHaveBeenCalledWith("create_config_from_template", {
      componentId: "vault",
      configPath: ".env",
      templatePath: ".env.example",
    });
  });
});

// The contract that matters most for a security tool: the wrappers that move
// CREDENTIALS, control the EGRESS ALLOWLIST, and drive the PERIMETER lifecycle.
// A wrong command name or arg shape here is a security/data-integrity failure
// (the shipped v0.6 first-run dead-end was exactly this class — credentials sent
// to the wrong IPC target). Pin each one.
describe("IPC contract: security-critical + lifecycle commands", () => {
  test("saveCredentials → save_credentials with both keys (NOT a config-dir write)", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await saveCredentials("sk-ant-xyz", "123:AAH");
    expect(mockInvoke).toHaveBeenCalledWith("save_credentials", {
      anthropicKey: "sk-ant-xyz",
      telegramToken: "123:AAH",
    });
  });

  test("readRuntimeEnv → read_runtime_env (no args)", async () => {
    mockInvoke.mockResolvedValue("ANTHROPIC_API_KEY=x\n");
    await readRuntimeEnv();
    expect(mockInvoke).toHaveBeenCalledWith("read_runtime_env");
  });

  test("validateAnthropicKey → validate_anthropic_key with the key", async () => {
    mockInvoke.mockResolvedValue({ ok: true });
    await validateAnthropicKey("sk-ant-secret");
    expect(mockInvoke).toHaveBeenCalledWith("validate_anthropic_key", { key: "sk-ant-secret" });
  });

  test("deriveTelegramBotUrl → derive_telegram_bot_url with the token", async () => {
    mockInvoke.mockResolvedValue({ url: "u", username: "n" });
    await deriveTelegramBotUrl("42:secret");
    expect(mockInvoke).toHaveBeenCalledWith("derive_telegram_bot_url", { token: "42:secret" });
  });

  test("applyAllowlistDecision → apply_allowlist_decision with host + decision", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await applyAllowlistDecision("evil.example.com", "deny");
    expect(mockInvoke).toHaveBeenCalledWith("apply_allowlist_decision", {
      host: "evil.example.com",
      decision: "deny",
    });
    await applyAllowlistDecision("api.anthropic.com", "always");
    expect(mockInvoke).toHaveBeenLastCalledWith("apply_allowlist_decision", {
      host: "api.anthropic.com",
      decision: "always",
    });
  });

  test("listEgressApprovals → list_egress_approvals (no args)", async () => {
    mockInvoke.mockResolvedValue([]);
    await listEgressApprovals();
    expect(mockInvoke).toHaveBeenCalledWith("list_egress_approvals");
  });

  test("perimeter lifecycle → restart/pause/resume/get_perimeter_state", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await restartPerimeter();
    expect(mockInvoke).toHaveBeenCalledWith("restart_perimeter");
    await pausePerimeter();
    expect(mockInvoke).toHaveBeenCalledWith("pause_perimeter");
    await resumePerimeter();
    expect(mockInvoke).toHaveBeenCalledWith("resume_perimeter");
    mockInvoke.mockResolvedValue({ state: "ok" });
    await getPerimeterState();
    expect(mockInvoke).toHaveBeenCalledWith("get_perimeter_state");
  });

  test("listWorkflows → list_workflows with componentId", async () => {
    mockInvoke.mockResolvedValue([]);
    await listWorkflows("agent");
    expect(mockInvoke).toHaveBeenCalledWith("list_workflows", { componentId: "agent" });
  });

  test("executeWorkflow → execute_workflow with componentId, workflowId, inputs (default {})", async () => {
    mockInvoke.mockResolvedValue({ status: "completed" });
    await executeWorkflow("agent", "full-verify");
    expect(mockInvoke).toHaveBeenCalledWith("execute_workflow", {
      componentId: "agent",
      workflowId: "full-verify",
      inputs: {},
    });
    await executeWorkflow("skills", "scan", { url: "https://x" });
    expect(mockInvoke).toHaveBeenLastCalledWith("execute_workflow", {
      componentId: "skills",
      workflowId: "scan",
      inputs: { url: "https://x" },
    });
  });

  test("telegram waker channel → delete_webhook / poll_for_start / send / advance_offset", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await telegramDeleteWebhook("tok");
    expect(mockInvoke).toHaveBeenCalledWith("telegram_delete_webhook", { token: "tok" });
    await telegramPollForStart("tok", 7, 30);
    expect(mockInvoke).toHaveBeenCalledWith("telegram_poll_for_start", {
      token: "tok",
      offset: 7,
      timeoutSecs: 30,
    });
    await telegramSendMessage("tok", 99, "hello");
    expect(mockInvoke).toHaveBeenCalledWith("telegram_send_message", {
      token: "tok",
      chatId: 99,
      text: "hello",
    });
    await telegramAdvanceOffset("tok", 12);
    expect(mockInvoke).toHaveBeenCalledWith("telegram_advance_offset", { token: "tok", updateId: 12 });
  });

  test("sentinel → get_sentinel_activity / sentinel_judge", async () => {
    mockInvoke.mockResolvedValue({});
    await getSentinelActivity();
    expect(mockInvoke).toHaveBeenCalledWith("get_sentinel_activity");
    const request = { host: "x", reason: "y" };
    mockInvoke.mockResolvedValue({ verdict: "allow" });
    await sentinelJudge(request);
    expect(mockInvoke).toHaveBeenCalledWith("sentinel_judge", { request });
  });
});

// The de-Tauri loopback transport (ADR-0022): in a plain browser the SAME wrappers POST to
// `/api/<cmd>` instead of using the Tauri IPC. The command-name + arg contract above is unchanged;
// these pin the browser transport — the URL, the JSON body, the cookie-carrying credentials, and
// the `{ error }` → thrown-Error mapping that gives callers the same rejection shape.
describe("browser transport: invoke → fetch /api/<cmd>", () => {
  const savedTauri = (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;

  beforeEach(() => {
    // browser mode: no Tauri runtime, so the wrappers take the fetch path
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });
  afterEach(() => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = savedTauri;
    vi.unstubAllGlobals();
  });

  test("POSTs /api/<cmd> with same-origin credentials and returns parsed JSON", async () => {
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(
      new Response(JSON.stringify([{ id: "agent" }]), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const result = await listComponents();
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/list_components",
      expect.objectContaining({ method: "POST", credentials: "same-origin" }),
    );
    expect(result).toEqual([{ id: "agent" }]);
  });

  test("named args become the JSON request body", async () => {
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(new Response("{}", { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);

    await getComponent("agent");
    expect(fetchMock.mock.calls[0]?.[1]?.body).toBe(JSON.stringify({ componentId: "agent" }));
  });

  test("a non-2xx { error } response is rethrown as Error(message) — same rejection shape as IPC", async () => {
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(
      new Response(JSON.stringify({ error: "component not found: x" }), { status: 404 }),
    );
    vi.stubGlobal("fetch", fetchMock);

    await expect(getComponent("x")).rejects.toThrow("component not found: x");
  });

  test("a void command resolves on a 200 null/empty body", async () => {
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(new Response("null", { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);

    // write_config returns () → null body → the wrapper resolves (callers ignore the value)
    await expect(writeConfig("agent", "c.yml", "x")).resolves.toBeNull();
  });
});
