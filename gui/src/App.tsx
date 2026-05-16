import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ModuleInfo {
  name: string;
  display_name: string;
  category: string;
  versions: string[];
  active_version: string | null;
}

interface ServiceInfo {
  name: string;
  status: string;
  pid: number | null;
  port: number | null;
}

function App() {
  const [modules, setModules] = useState<ModuleInfo[]>([]);
  const [services, setServices] = useState<ServiceInfo[]>([]);
  const [platform, setPlatform] = useState("");
  const [selected, setSelected] = useState<string | null>(null);
  const [statusMsg, setStatusMsg] = useState("");

  useEffect(() => {
    invoke<ModuleInfo[]>("list_modules").then(setModules);
    invoke<ServiceInfo[]>("get_services").then(setServices);
    invoke<string>("get_platform").then(setPlatform);
  }, []);

  const refresh = () => {
    invoke<ModuleInfo[]>("list_modules").then(setModules);
    invoke<ServiceInfo[]>("get_services").then(setServices);
  };

  const cover = async (mod: string, ver: string) => {
    try {
      await invoke("cover_module", { module: mod, version: ver, global: false });
      setStatusMsg(`${mod} ${ver} covered`);
      refresh();
    } catch (e) {
      setStatusMsg(`Error: ${e}`);
    }
  };

  const uncover = async (mod: string) => {
    try {
      await invoke("uncover_module", { module: mod });
      setStatusMsg(`${mod} uncovered`);
      refresh();
    } catch (e) {
      setStatusMsg(`Error: ${e}`);
    }
  };

  const toggleService = async (name: string, version: string | null, running: boolean) => {
    try {
      if (running) {
        await invoke("stop_service", { module: name });
        setStatusMsg(`${name} stopped`);
      } else if (version) {
        await invoke("start_service", { module: name, version: version });
        setStatusMsg(`${name} started`);
      }
      refresh();
    } catch (e) {
      setStatusMsg(`Error: ${e}`);
    }
  };

  const selectedModule = modules.find((m) => m.name === selected);

  return (
    <div className="h-screen flex flex-col bg-zinc-950 text-zinc-200">
      <header className="flex items-center justify-between px-4 py-3 border-b border-zinc-800">
        <div className="flex items-center gap-3">
          <h1 className="text-lg font-bold text-white">envSwitch</h1>
          <span className="text-xs text-zinc-500">{platform}</span>
        </div>
        <button onClick={refresh} className="px-3 py-1 text-sm bg-zinc-800 hover:bg-zinc-700 rounded-md">
          Refresh
        </button>
      </header>

      {statusMsg && (
        <div className="px-4 py-2 text-sm bg-emerald-900/50 border-b border-emerald-800">{statusMsg}</div>
      )}

      <div className="flex flex-1 overflow-hidden">
        <nav className="w-56 border-r border-zinc-800 overflow-y-auto p-2 space-y-1">
          {modules.map((m) => (
            <button
              key={m.name}
              onClick={() => setSelected(m.name)}
              className={`w-full text-left px-3 py-2 rounded-md text-sm transition-colors ${
                selected === m.name ? "bg-zinc-700 text-white" : "hover:bg-zinc-800 text-zinc-400"
              }`}
            >
              <div className="font-medium">{m.display_name}</div>
              <div className="text-xs text-zinc-500">{m.name} · {m.category}</div>
              {m.active_version && <span className="text-xs text-emerald-400">{m.active_version} ✓</span>}
            </button>
          ))}
        </nav>

        <main className="flex-1 overflow-y-auto p-4">
          {selectedModule ? (
            <div>
              <h2 className="text-xl font-bold mb-4">{selectedModule.display_name}</h2>
              <div className="space-y-2">
                {selectedModule.versions.length === 0 && (
                  <p className="text-zinc-600 text-sm">No versions installed</p>
                )}
                {selectedModule.versions.map((ver) => (
                  <div
                    key={ver}
                    className={`flex items-center justify-between px-3 py-2 rounded-md text-sm border ${
                      selectedModule.active_version === ver
                        ? "border-emerald-700 bg-emerald-950/50"
                        : "border-zinc-800 bg-zinc-900"
                    }`}
                  >
                    <span>
                      {ver}
                      {selectedModule.active_version === ver && (
                        <span className="ml-2 text-emerald-400 text-xs">active</span>
                      )}
                    </span>
                    <div className="flex gap-1">
                      <button
                        onClick={() => cover(selectedModule.name, ver)}
                        disabled={selectedModule.active_version === ver}
                        className="px-2 py-1 text-xs bg-emerald-800 hover:bg-emerald-700 rounded disabled:opacity-50"
                      >
                        Cover
                      </button>
                      {selectedModule.active_version === ver && (
                        <button
                          onClick={() => uncover(selectedModule.name)}
                          className="px-2 py-1 text-xs bg-zinc-700 hover:bg-zinc-600 rounded"
                        >
                          Uncover
                        </button>
                      )}
                    </div>
                  </div>
                ))}
              </div>

              {selectedModule.category === "Service" && (
                <div className="mt-6">
                  <h3 className="text-sm font-semibold mb-2">Service</h3>
                  {(() => {
                    const svc = services.find((s) => s.name === selectedModule.name);
                    const running = svc?.status === "Running";
                    return (
                      <div className="flex items-center gap-3">
                        <span className={`w-2 h-2 rounded-full ${running ? "bg-emerald-400" : "bg-zinc-600"}`} />
                        <span className="text-sm">{running ? `PID ${svc?.pid} | Port ${svc?.port}` : "Stopped"}</span>
                        <button
                          onClick={() => toggleService(selectedModule.name, selectedModule.active_version, running)}
                          className={`px-3 py-1 text-xs rounded ${running ? "bg-red-800 hover:bg-red-700" : "bg-emerald-800 hover:bg-emerald-700"}`}
                        >
                          {running ? "Stop" : "Start"}
                        </button>
                      </div>
                    );
                  })()}
                </div>
              )}
            </div>
          ) : (
            <div className="flex items-center justify-center h-full text-zinc-600">Select a module</div>
          )}
        </main>
      </div>
    </div>
  );
}

export default App;
