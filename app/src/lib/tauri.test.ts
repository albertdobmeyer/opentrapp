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
