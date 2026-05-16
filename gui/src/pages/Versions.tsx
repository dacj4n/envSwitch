import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';
import TopBar from '../components/TopBar';

interface ModuleInfo {
  name: string; display_name: string; category: string;
  versions: string[]; active_version: string | null;
}

export default function VersionsPage() {
  const { t } = useTranslation();
  const [modules, setModules] = useState<ModuleInfo[]>([]);

  const refresh = () => {
    // Sync silently, then list
    invoke('sync_local').then(() =>
      invoke<ModuleInfo[]>('list_modules').then(setModules)
    ).catch(() =>
      invoke<ModuleInfo[]>('list_modules').then(setModules)
    );
  };
  useEffect(() => { refresh(); }, []);

  const cover = async (mod: string, ver: string) => {
    try {
      await invoke('cover_module', { module: mod, version: ver, global: false });
      toast.success(`${mod} ${ver} ${t('toast.covered')}`);
      refresh();
    } catch (e) { toast.error(`${t('toast.error')}: ${e}`); }
  };

  const uncover = async (mod: string) => {
    try {
      await invoke('uncover_module', { module: mod });
      toast.success(`${mod} ${t('toast.uncovered')}`);
      refresh();
    } catch (e) { toast.error(`${t('toast.error')}: ${e}`); }
  };

  return (
    <div className="flex flex-col h-full">
      <TopBar title={t('nav.versions')} subtitle="Cover / Uncover module versions" onRefresh={refresh} />
      <div className="flex-1 overflow-y-auto p-4">
        <div className="grid grid-cols-2 gap-4">
          {modules.map((m) => (
            <div key={m.name} className="rounded-xl border border-border bg-card overflow-hidden">
              <div className="flex items-center gap-2 px-4 py-3 border-b border-border">
                <span className="text-xs px-2 py-0.5 rounded bg-primary/10 text-primary font-mono">{m.name}</span>
                <span className="font-medium text-sm">{m.display_name}</span>
                <span className="text-xs text-muted-foreground">{m.category}</span>
              </div>
              <div className="p-4 space-y-2">
                {m.versions.length === 0 && (
                  <p className="text-xs text-muted-foreground">{t('common.noVersions')}</p>
                )}
                {m.versions.map((ver) => {
                  const isActive = m.active_version === ver;
                  return (
                    <div key={ver} className={`flex items-center justify-between px-3 py-2 rounded-md text-xs border ${
                      isActive ? 'border-primary/40 bg-primary/5' : 'border-border'
                    }`}>
                      <span className="font-mono">{ver}</span>
                      <div className="flex gap-1">
                        <button
                          onClick={() => cover(m.name, ver)}
                          disabled={isActive}
                          className="px-2 py-1 rounded bg-primary/20 hover:bg-primary/30 text-primary disabled:opacity-30 disabled:cursor-not-allowed text-[11px]"
                        >
                          {t('common.cover')}
                        </button>
                        {isActive && (
                          <button onClick={() => uncover(m.name)}
                            className="px-2 py-1 rounded bg-secondary hover:bg-secondary/80 text-secondary-foreground text-[11px]"
                          >
                            {t('common.uncover')}
                          </button>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
