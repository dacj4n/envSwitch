import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';
import TopBar from '../components/TopBar';
import { PlayIcon, SquareIcon, ServerIcon, HardDriveIcon, NetworkIcon, CircleDotIcon, DatabaseIcon, Loader2Icon } from 'lucide-react';

interface ServiceInfo { name: string; status: string; pid: number | null; port: number | null; }
interface ModuleInfo { name: string; display_name: string; category: string; versions: string[]; active_version: string | null; source_paths: string[]; }

const MODULE_COLORS: Record<string, string> = { mysql: '#f97316', pgsql: '#60a5fa' };
const MODULE_LABELS: Record<string, string> = { mysql: 'MySQL', pgsql: 'PostgreSQL' };

export default function ServicesPage() {
  const { t } = useTranslation();
  const [services, setServices] = useState<ServiceInfo[]>([]);
  const [modules, setModules] = useState<ModuleInfo[]>([]);
  const [toggling, setToggling] = useState<Set<string>>(new Set());

  const STATUS_CONFIG: Record<string, { dot: string; text: string; badge: string }> = {
    Running: { dot: 'bg-success animate-pulse-slow', text: 'text-success', badge: 'bg-success/10 border-success/30' },
    Stopped: { dot: 'bg-muted-foreground', text: 'text-muted-foreground', badge: 'bg-muted/30 border-border' },
  };

  // Listen for job updates to detect start/stop completion
  useEffect(() => {
    const unlisten = listen<{id: string; kind: string; module: string; status: string; message: string}>('job-update', (ev) => {
      const j = ev.payload;
      if (j.kind === 'start' || j.kind === 'stop') {
        if (j.status === 'success' || j.status === 'failed') {
          if (j.status === 'success') toast.success(j.message);
          else toast.error(j.message);
          refresh();
          setToggling(prev => {
            const next = new Set(prev);
            next.delete(j.module);
            return next;
          });
        }
      }
    });
    return () => { unlisten.then(f => f()); };
  }, []);

  const refresh = useCallback(() => {
    invoke<ServiceInfo[]>('get_services').then(setServices);
    invoke<ModuleInfo[]>('list_modules').then(setModules);
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  const toggle = (name: string, running: boolean) => {
    const mod = modules.find(m => m.name === name);
    const ver = mod?.active_version || mod?.versions[0];
    setToggling(prev => new Set(prev).add(name));
    const cmd = running
      ? invoke('stop_service', { module: name })
      : invoke('start_service', { module: name, version: ver });
    cmd.catch(e => {
      toast.error(`${e}`);
      setToggling(prev => {
        const next = new Set(prev);
        next.delete(name);
        return next;
      });
    });
  };

  const runningCount = services.filter(s => s.status === 'Running').length;
  const stoppedCount = services.filter(s => s.status === 'Stopped').length;

  return (
    <div className="flex flex-col h-full min-h-0">
      <TopBar title={t('nav.services')} subtitle={t('service.manageSubtitle')} onRefresh={refresh} />

      <div className="flex-1 overflow-y-auto px-5 py-4">
        {/* Status overview */}
        <div className="flex items-center gap-3 mb-5">
          <div className="flex items-center gap-2 px-4 py-2.5 rounded-lg bg-card border border-border">
            <DatabaseIcon className="w-4 h-4 text-muted-foreground" />
            <span className="text-sm text-muted-foreground">{services.length} {t('nav.services')}</span>
          </div>
          <div className="flex items-center gap-2 px-4 py-2.5 rounded-lg bg-success/10 border border-success/25">
            <span className="w-2 h-2 rounded-full bg-success animate-pulse-slow" />
            <span className="text-sm text-success font-medium">{runningCount} {t('service.running')}</span>
          </div>
          <div className="flex items-center gap-2 px-4 py-2.5 rounded-lg bg-card border border-border">
            <span className="w-2 h-2 rounded-full bg-muted-foreground" />
            <span className="text-sm text-muted-foreground">{stoppedCount} {t('service.stopped')}</span>
          </div>
          <div className="ml-auto flex items-center gap-2">
            <button onClick={() => {
              for (const s of services) { if (s.status === 'Stopped') toggle(s.name, false); }
            }} className="flex items-center gap-1.5 px-3 py-2 rounded-md text-xs bg-success/10 text-success hover:bg-success/20 border border-success/30 font-medium"
            ><PlayIcon className="w-3.5 h-3.5" /> {t('common.startAll')}</button>
            <button onClick={() => {
              for (const s of services) { if (s.status === 'Running') toggle(s.name, true); }
            }} className="flex items-center gap-1.5 px-3 py-2 rounded-md text-xs bg-destructive/10 text-destructive hover:bg-destructive/20 border border-destructive/30 font-medium"
            ><SquareIcon className="w-3.5 h-3.5" /> {t('common.stopAll')}</button>
          </div>
        </div>

        {/* Service cards */}
        <div className="flex flex-col gap-4">
          {services.map(svc => {
            const mod = modules.find(m => m.name === svc.name);
            const running = svc.status === 'Running';
            const color = MODULE_COLORS[svc.name] ?? '#888';
            const label = MODULE_LABELS[svc.name] ?? svc.name;
            const cfg = STATUS_CONFIG[svc.status] ?? STATUS_CONFIG.Stopped;
            const activeVer = mod?.active_version || mod?.versions[0];

            return (
              <div key={svc.name} className="rounded-xl border border-border bg-card overflow-hidden">
                {/* Header */}
                <div className="flex items-center justify-between px-5 py-4 border-b border-border">
                  <div className="flex items-center gap-3">
                    <div className="w-9 h-9 rounded-lg flex items-center justify-center"
                      style={{ backgroundColor: `${color}18`, border: `1px solid ${color}30` }}>
                      <ServerIcon className="w-4 h-4" style={{ color }} />
                    </div>
                    <div>
                      <div className="flex items-center gap-2">
                        <span className="font-semibold text-sm text-foreground">{label}</span>
                        {activeVer && <span className="text-xs font-mono text-muted-foreground">v{activeVer}</span>}
                      </div>
                      <div className="flex items-center gap-1.5 mt-0.5">
                        <span className={`w-2 h-2 rounded-full ${cfg.dot}`} />
                        <span className={`text-xs ${cfg.text}`}>{svc.status === 'Running' ? t('service.running') : t('service.stopped')}</span>
                      </div>
                    </div>
                  </div>
                  <div className={`px-3 py-1.5 rounded-full border text-xs font-medium ${cfg.badge} ${cfg.text}`}>
                    <div className="flex items-center gap-1.5"><CircleDotIcon className="w-3 h-3" />{svc.status === 'Running' ? t('service.running') : t('service.stopped')}</div>
                  </div>
                </div>

                {/* Metadata */}
                <div className="px-5 py-4 grid gap-3">
                  <div className="flex items-center gap-4">
                    {svc.port && (
                      <div className="flex items-center gap-2 text-xs text-muted-foreground">
                        <NetworkIcon className="w-3.5 h-3.5 text-muted-foreground/60" />
                        <span className="font-mono">:{svc.port}</span>
                      </div>
                    )}
                    {svc.pid && (
                      <div className="flex items-center gap-2 text-xs text-muted-foreground">
                        <HardDriveIcon className="w-3.5 h-3.5 text-muted-foreground/60" />
                        <span className="font-mono">PID {svc.pid}</span>
                      </div>
                    )}
                  </div>
                  <div className="flex items-center gap-2 text-xs">
                    <HardDriveIcon className="w-3.5 h-3.5 text-muted-foreground/60 shrink-0" />
                    <span className="font-mono text-muted-foreground truncate">~/.envswitch/data/{svc.name}/{activeVer || '...'}</span>
                  </div>
                </div>

                {/* Actions */}
                <div className="flex items-center gap-2 px-5 pb-4">
                  <button onClick={() => toggle(svc.name, false)} disabled={running || toggling.has(svc.name)}
                    className="flex items-center gap-1.5 px-3 py-2 rounded-md text-xs bg-success/10 text-success hover:bg-success/20 border border-success/30 font-medium disabled:opacity-40"
                  >{toggling.has(svc.name) && !running ? <Loader2Icon className="w-3.5 h-3.5 animate-spin" /> : <PlayIcon className="w-3.5 h-3.5" />} {t('common.start')}</button>
                  <button onClick={() => toggle(svc.name, true)} disabled={!running || toggling.has(svc.name)}
                    className="flex items-center gap-1.5 px-3 py-2 rounded-md text-xs bg-destructive/10 text-destructive hover:bg-destructive/20 border border-destructive/30 font-medium disabled:opacity-40"
                  >{toggling.has(svc.name) && running ? <Loader2Icon className="w-3.5 h-3.5 animate-spin" /> : <SquareIcon className="w-3.5 h-3.5" />} {t('common.stop')}</button>
                  <div className="ml-auto text-[10px] font-mono text-muted-foreground/60">
                    $ envswitch {running ? 'stop' : 'start'} {svc.name}
                  </div>
                </div>
              </div>
            );
          })}
        </div>

        {/* CLI Quick Reference */}
        <div className="mt-6 rounded-xl border border-border bg-card overflow-hidden">
          <div className="px-5 py-3.5 border-b border-border">
            <span className="text-sm font-semibold text-foreground">{t('service.cliRef')}</span>
          </div>
          <div className="p-5 grid gap-2">
            {[
              ['Start MySQL', 'envswitch start mysql 8.0.46'],
              ['Stop MySQL', 'envswitch stop mysql'],
              ['Start PostgreSQL', 'envswitch start pgsql 16.14'],
              ['Stop PostgreSQL', 'envswitch stop pgsql'],
              ['View service logs', 'envswitch logs mysql --lines 100'],
              ['Check all services', 'envswitch service-status'],
            ].map(([label, cmd]) => (
              <div key={cmd} className="flex items-center gap-4">
                <span className="text-xs text-muted-foreground w-36 shrink-0">{label}</span>
                <code className="flex-1 font-mono text-xs px-3 py-1.5 rounded-md bg-muted/30 text-accent-foreground">
                  $ {cmd}
                </code>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
