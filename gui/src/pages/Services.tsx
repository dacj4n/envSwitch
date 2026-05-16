import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';
import TopBar from '../components/TopBar';

interface ServiceInfo { name: string; status: string; pid: number | null; port: number | null; }
interface ModuleInfo { name: string; display_name: string; category: string; versions: string[]; active_version: string | null; }

export default function ServicesPage() {
  const { t } = useTranslation();
  const [services, setServices] = useState<ServiceInfo[]>([]);
  const [modules, setModules] = useState<ModuleInfo[]>([]);

  const refresh = () => {
    invoke<ServiceInfo[]>('get_services').then(setServices);
    invoke<ModuleInfo[]>('list_modules').then(setModules);
  };
  useEffect(() => { refresh(); }, []);

  const toggle = async (name: string, running: boolean) => {
    const mod = modules.find(m => m.name === name);
    const ver = mod?.active_version || mod?.versions[0];
    try {
      if (running) {
        await invoke('stop_service', { module: name });
        toast.success(`${name} ${t('toast.stopped')}`);
      } else if (ver) {
        await invoke('start_service', { module: name, version: ver });
        toast.success(`${name} ${t('toast.started')}`);
      }
      refresh();
    } catch (e) { toast.error(`${t('toast.error')}: ${e}`); }
  };

  return (
    <div className="flex flex-col h-full">
      <TopBar title={t('nav.services')} subtitle="Start / Stop database services" onRefresh={refresh} />
      <div className="flex-1 overflow-y-auto p-4">
        <div className="space-y-3">
          {services.map((svc) => {
            const running = svc.status === 'Running';
            return (
              <div key={svc.name} className="flex items-center justify-between px-4 py-3 rounded-xl border border-border bg-card">
                <div className="flex items-center gap-3">
                  <span className={`w-2.5 h-2.5 rounded-full ${running ? 'bg-emerald-400 shadow-emerald-400/50 shadow-sm' : 'bg-muted-foreground/40'}`} />
                  <span className="font-medium text-sm">{svc.name}</span>
                  <span className={`text-xs ${running ? 'text-emerald-400' : 'text-muted-foreground'}`}>
                    {running ? `${t('service.pid')} ${svc.pid} · ${t('service.port')} ${svc.port}` : t('service.stopped')}
                  </span>
                </div>
                <button
                  onClick={() => toggle(svc.name, running)}
                  className={`px-4 py-1.5 text-xs rounded-md font-medium transition-colors ${
                    running ? 'bg-red-500/15 text-red-400 hover:bg-red-500/25' : 'bg-emerald-500/15 text-emerald-400 hover:bg-emerald-500/25'
                  }`}
                >
                  {running ? t('common.stop') : t('common.start')}
                </button>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
