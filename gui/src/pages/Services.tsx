import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';
import TopBar from '../components/TopBar';
import { PlayIcon, SquareIcon, ChevronDownIcon, ChevronRightIcon, CircleIcon, FolderIcon, FileTextIcon, TerminalIcon } from 'lucide-react';

interface ServiceInfo { name: string; status: string; pid: number | null; port: number | null; }
interface ModuleInfo { name: string; display_name: string; category: string; versions: string[]; active_version: string | null; }

const MODULE_COLORS: Record<string, string> = { mysql: '#f97316', pgsql: '#60a5fa' };
const MODULE_ICONS: Record<string, string> = { mysql: '🐬', pgsql: '🐘' };

export default function ServicesPage() {
  const { t } = useTranslation();
  const [services, setServices] = useState<ServiceInfo[]>([]);
  const [modules, setModules] = useState<ModuleInfo[]>([]);
  const [expanded, setExpanded] = useState<string | null>(null);

  const refresh = () => {
    invoke<ServiceInfo[]>('get_services').then(setServices);
    invoke<ModuleInfo[]>('list_modules').then(setModules);
  };
  useEffect(() => { refresh(); }, []);

  const toggle = async (name: string, running: boolean) => {
    const mod = modules.find(m => m.name === name);
    const ver = mod?.active_version || mod?.versions[0];
    try {
      if (running) { await invoke('stop_service', { module: name }); toast.success(`${name} stopped`); }
      else if (ver) { await invoke('start_service', { module: name, version: ver }); toast.success(`${name} started`); }
      refresh();
    } catch (e) { toast.error(`${e}`); }
  };

  return (
    <div className="flex flex-col h-full">
      <TopBar title={t('nav.services')} subtitle="Start / Stop database services" onRefresh={refresh} />
      <div className="flex-1 overflow-y-auto px-5 py-4 space-y-3">
        {services.map(svc => {
          const mod = modules.find(m => m.name === svc.name);
          const running = svc.status === 'Running';
          const color = MODULE_COLORS[svc.name] ?? '#888';
          const icon = MODULE_ICONS[svc.name] ?? '📦';
          const open = expanded === svc.name;

          return (
            <div key={svc.name} className="rounded-xl border border-border bg-card overflow-hidden transition-all duration-200 hover:border-border/80">
              <button onClick={() => setExpanded(open ? null : svc.name)}
                className="w-full flex items-center gap-3 px-5 py-4 hover:bg-muted/30 transition-colors text-left"
              >
                <div className="w-9 h-9 rounded-lg flex items-center justify-center text-base shrink-0"
                  style={{ backgroundColor: `${color}18`, border: `1px solid ${color}30` }}>
                  <span>{icon}</span>
                </div>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="font-semibold text-sm text-foreground">{svc.name}</span>
                    {running && (
                      <span className="text-[10px] px-1.5 py-0.5 rounded-md bg-success/15 text-success border border-success/25">RUNNING</span>
                    )}
                  </div>
                  <div className="text-xs text-muted-foreground mt-0.5">
                    {running ? `PID ${svc.pid} · Port ${svc.port}` : 'Stopped'}
                    {mod && ` · ${mod.versions.length} versions`}
                  </div>
                </div>
                <div className="flex items-center gap-2 shrink-0">
                  <button onClick={(e) => { e.stopPropagation(); toggle(svc.name, running); }}
                    className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium border transition-all ${
                      running
                        ? 'bg-destructive/10 text-destructive border-destructive/30 hover:bg-destructive/20'
                        : 'bg-success/10 text-success border-success/30 hover:bg-success/20'
                    }`}
                  >
                    {running ? <SquareIcon className="w-3 h-3" /> : <PlayIcon className="w-3 h-3" />}
                    {running ? 'Stop' : 'Start'}
                  </button>
                  <div className="text-muted-foreground">
                    {open ? <ChevronDownIcon className="w-4 h-4" /> : <ChevronRightIcon className="w-4 h-4" />}
                  </div>
                </div>
              </button>

              {open && mod && (
                <div className="border-t border-border">
                  <div className="px-5 py-3 border-b border-border/50 bg-muted/5 flex items-center gap-4 text-[10px] text-muted-foreground uppercase tracking-wider">
                    <span className="w-40">Version</span>
                    <span>Status</span>
                    <span className="flex-1">Data Directory</span>
                    <span>Actions</span>
                  </div>
                  {mod.versions.map((ver, _idx) => {
                    const isActive = mod.active_version === ver;
                    return (
                      <div key={ver} className={`flex items-center gap-4 px-5 py-3 border-b border-border/50 last:border-b-0 transition-colors hover:bg-muted/20`}>
                        <div className="w-40 flex items-center gap-2">
                          {isActive ? <div className="w-1.5 h-1.5 rounded-full bg-success" /> : <CircleIcon className="w-3 h-3 text-border" />}
                          <span className={`font-mono text-sm font-medium ${isActive ? 'text-success' : 'text-foreground'}`}>v{ver}</span>
                        </div>
                        <span className={`text-xs ${isActive ? 'text-success' : 'text-muted-foreground'}`}>
                          {isActive ? 'active' : 'installed'}
                        </span>
                        <div className="flex-1 flex items-center gap-2 text-[11px] text-muted-foreground font-mono">
                          <FolderIcon className="w-3 h-3 shrink-0" />
                          <span className="truncate">~/.envswitch/data/{svc.name}/{ver}</span>
                        </div>
                        <div className="flex items-center gap-1.5 shrink-0">
                          <button className="flex items-center gap-1 px-2 py-1 rounded-md text-[11px] text-muted-foreground hover:text-foreground hover:bg-muted/30 border border-transparent hover:border-border transition-all"
                            title="Open data directory">
                            <FolderIcon className="w-3 h-3" />
                          </button>
                          <button className="flex items-center gap-1 px-2 py-1 rounded-md text-[11px] text-muted-foreground hover:text-foreground hover:bg-muted/30 border border-transparent hover:border-border transition-all"
                            title="View logs">
                            <FileTextIcon className="w-3 h-3" />
                          </button>
                          <button className="flex items-center gap-1 px-2 py-1 rounded-md text-[11px] text-muted-foreground hover:text-foreground hover:bg-muted/30 border border-transparent hover:border-border transition-all"
                            title="Binary location">
                            <TerminalIcon className="w-3 h-3" />
                          </button>
                        </div>
                      </div>
                    );
                  })}
                  {mod.versions.length === 0 && (
                    <div className="px-5 py-4 text-xs text-muted-foreground">No versions installed</div>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
