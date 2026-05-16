import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import TopBar from '../components/TopBar';
import { CheckCircleIcon, CircleIcon, ArrowRightIcon } from 'lucide-react';

interface ActiveCover { module_name: string; version: string; scope: string; applied_at: string; }

const MODULE_COLORS: Record<string, string> = {
  jdk: '#f59e0b', go: '#22d3ee', node: '#22c55e', php: '#818cf8',
  python: '#facc15', mysql: '#f97316', pgsql: '#60a5fa',
};
const MODULE_ICONS: Record<string, string> = {
  jdk: '☕', go: '🐹', node: '🟢', php: '🐘', python: '🐍', mysql: '🐬', pgsql: '🐘',
};

export default function StatusPage() {
  const { t } = useTranslation();
  const [covers, setCovers] = useState<ActiveCover[]>([]);
  const [allModules, setAllModules] = useState<string[]>([]);

  const refresh = () => {
    invoke<ActiveCover[]>('get_status').then(setCovers);
    // Get all module names for full table
    invoke<{name:string}[]>('list_modules').then(list => setAllModules(list.map((m: any) => m.name)));
  };
  useEffect(() => { refresh(); }, []);

  // Build status list: covered modules + non-covered
  const coveredSet = new Set(covers.map(c => c.module_name));
  const statusList: { module: string; activeVersion: string | null; scope: string; isActive: boolean }[] = covers.map(c => ({
    module: c.module_name, activeVersion: c.version, scope: c.scope, isActive: true
  }));
  // Add non-covered modules
  for (const name of allModules) {
    if (!coveredSet.has(name)) {
      statusList.push({ module: name, activeVersion: null, scope: '', isActive: false });
    }
  }

  return (
    <div className="flex flex-col h-full min-h-0">
      <TopBar title={t('nav.status')} subtitle="Active version covers" onRefresh={refresh} />

      <div className="flex-1 overflow-y-auto px-5 py-4">
        {statusList.length === 0 ? (
          <div className="flex items-center justify-center h-32 text-sm text-muted-foreground">No active covers</div>
        ) : (
          <div className="rounded-xl border border-border bg-card overflow-hidden">
            <div className="flex items-center gap-2 px-5 py-4 border-b border-border">
              <CheckCircleIcon className="w-4 h-4 text-primary" />
              <span className="font-semibold text-sm text-foreground">Environment Status</span>
              <span className="ml-auto text-xs text-muted-foreground font-mono">{covers.length} active</span>
            </div>
            <div className="divide-y divide-border/50">
              {statusList.map(s => {
                const color = MODULE_COLORS[s.module] ?? '#888';
                const icon = MODULE_ICONS[s.module] ?? '📦';
                return (
                  <div key={s.module} className="flex items-center gap-4 px-5 py-3.5 hover:bg-muted/10 transition-colors">
                    <div className="flex items-center gap-2.5 w-28 shrink-0">
                      <div className="w-6 h-6 rounded-md flex items-center justify-center text-xs"
                        style={{ backgroundColor: `${color}18` }}>
                        {icon}
                      </div>
                      <span className="text-sm font-mono font-medium text-foreground">{s.module}</span>
                    </div>
                    <ArrowRightIcon className="w-3.5 h-3.5 text-border shrink-0" />
                    <div className="w-24 shrink-0">
                      {s.activeVersion ? (
                        <span className="inline-block font-mono text-sm font-semibold px-2 py-0.5 rounded-md"
                          style={{ backgroundColor: `${color}15`, color }}>
                          v{s.activeVersion}
                        </span>
                      ) : (
                        <span className="inline-block font-mono text-sm text-muted-foreground px-2 py-0.5 rounded-md bg-muted/30">—</span>
                      )}
                    </div>
                    <div className="flex-1 min-w-0">
                      {s.activeVersion ? (
                        <span className="font-mono text-xs text-muted-foreground truncate block">
                          ~/.envswitch/shims/{s.module} → ~/.envswitch/envs/{s.module}/{s.activeVersion}
                        </span>
                      ) : (
                        <span className="font-mono text-xs text-muted-foreground/40 truncate block">not covered</span>
                      )}
                    </div>
                    <div className="shrink-0">
                      {s.isActive ? (
                        <span className="text-[10px] px-2 py-0.5 rounded-full bg-success/10 text-success border border-success/25 font-medium">
                          {s.scope}
                        </span>
                      ) : (
                        <span className="text-[10px] px-2 py-0.5 rounded-full bg-muted/30 text-muted-foreground border border-border font-medium">inactive</span>
                      )}
                    </div>
                    <div className="shrink-0 flex items-center justify-center">
                      {s.isActive ? <CheckCircleIcon className="w-4 h-4 text-success" /> : <CircleIcon className="w-4 h-4 text-border" />}
                    </div>
                  </div>
                );
              })}
            </div>
            <div className="px-5 py-3 border-t border-border/50 bg-muted/5">
              <span className="text-[11px] font-mono text-muted-foreground/60">
                Shims directory: ~/.envswitch/shims — ensure it's in your $PATH
              </span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
