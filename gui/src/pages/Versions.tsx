import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';
import TopBar from '../components/TopBar';
import {
  DownloadIcon, PlayIcon, XCircleIcon, Trash2Icon, Loader2Icon,
  ChevronDownIcon, ChevronRightIcon, CircleIcon, PackageIcon, CheckCircleIcon,
} from 'lucide-react';

interface ModuleInfo {
  name: string; display_name: string; category: string;
  versions: string[]; active_version: string | null;
  source_paths: string[]; is_symlinked: boolean[];
  uninstallable: boolean[];
  version_labels: string[];
}

const MODULE_COLORS: Record<string, string> = {
  jdk: '#f59e0b', go: '#22d3ee', node: '#22c55e', php: '#818cf8',
  python: '#facc15', mysql: '#f97316', pgsql: '#60a5fa',
};
const MODULE_ICONS: Record<string, string> = {
  jdk: '☕', go: '🐹', node: '🟢', php: '🐘', python: '🐍', mysql: '🐬', pgsql: '🐘',
};

export default function VersionsPage() {
  const { t } = useTranslation();
  const [modules, setModules] = useState<ModuleInfo[]>([]);
  const [expanded, setExpanded] = useState<string | null>(null);
  const [searchResults, setSearchResults] = useState<Record<string, string[]>>(() => {
    try { return JSON.parse(sessionStorage.getItem('ev_search') || '{}'); } catch { return {}; }
  });
  const [searching, setSearching] = useState<Record<string, boolean>>({});
  const [installing, setInstalling] = useState<string | null>(null);

  // Persist search results to sessionStorage
  const updateSearchResults = (r: Record<string, string[]>) => {
    setSearchResults(r);
    try { sessionStorage.setItem('ev_search', JSON.stringify(r)); } catch {}
  };

  const refresh = () => {
    invoke('sync_local').then(() =>
      invoke<ModuleInfo[]>('list_modules').then(setModules)
    ).catch(() =>
      invoke<ModuleInfo[]>('list_modules').then(setModules)
    );
  };
  useEffect(() => { refresh(); }, []);

  const cover = async (mod: string, ver: string) => {
    try { await invoke('cover_module', { module: mod, version: ver, global: false }); toast.success(`${mod} ${ver} covered`); refresh(); }
    catch (e) { toast.error(`${e}`); }
  };
  const uncover = async (mod: string) => {
    try { await invoke('uncover_module', { module: mod }); toast.success(`${mod} uncovered`); refresh(); }
    catch (e) { toast.error(`${e}`); }
  };
  // Listen for search results (async, non-blocking)
  useEffect(() => {
    const unlisten = listen<{module: string; versions: string[]}>('search-results', (ev) => {
      updateSearchResults({ ...searchResults, [ev.payload.module]: ev.payload.versions });
      setSearching(s => ({ ...s, [ev.payload.module]: false }));
    });
    return () => { unlisten.then(fn => fn()); };
  }, []);

  // Listen for job updates (install/uninstall progress)
  useEffect(() => {
    const unlisten = listen<{id: string; kind: string; module: string; version: string; status: string; progress: number; message: string}>('job-update', (ev) => {
      const p = ev.payload;
      if (p.status === 'success' || p.status === 'failed') {
        if (p.kind === 'install') toast.success(p.message);
        else if (p.kind === 'uninstall') toast.success(p.message);
        else if (p.status === 'failed') toast.error(p.message);
        setInstalling(null);
        refresh();
      }
    });
    return () => { unlisten.then(fn => fn()); };
  }, []);

  const doInstall = (mod: string, ver: string) => {
    setInstalling(`${mod}:${ver}`);
    invoke('install_version', { module: mod, version: ver }); // returns job_id immediately, events handle progress
  };
  const doUninstall = (mod: string, ver: string) => {
    invoke('uninstall_version', { module: mod, version: ver }); // async via job
  };

  const toggleModule = (mod: string) => {
    setExpanded(expanded === mod ? null : mod);
  };

  const fetchAvailable = (mod: string) => {
    setSearching(s => ({ ...s, [mod]: true }));
    invoke('search_versions', { module: mod }); // returns immediately, results via event
  };

  return (
    <div className="flex flex-col h-full min-h-0">
      <TopBar
        title={t('nav.versions')}
        subtitle={t('versions.subtitle')}
        onRefresh={refresh}
      />

      <div className="flex-1 overflow-y-auto px-5 py-4 space-y-3">
        {modules.map(m => {
          const color = MODULE_COLORS[m.name] ?? '#888';
          const icon = MODULE_ICONS[m.name] ?? '📦';
          const open = expanded === m.name;
          const results = searchResults[m.name] || [];
          const isSearching = searching[m.name] || false;
          const installedSet = new Set(m.versions);
          const installedCount = m.versions.length;

          return (
            <div key={m.name}
              className="rounded-xl border border-border bg-card overflow-hidden transition-all duration-200 hover:border-border/80"
            >
              {/* Header */}
              <button onClick={() => toggleModule(m.name)}
                className="w-full flex items-center gap-3 px-5 py-4 hover:bg-muted/30 transition-colors text-left"
              >
                <div className="w-9 h-9 rounded-lg flex items-center justify-center text-base shrink-0"
                  style={{ backgroundColor: `${color}18`, border: `1px solid ${color}30` }}>
                  <span>{icon}</span>
                </div>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="font-semibold text-sm text-foreground">{m.display_name}</span>
                    {m.active_version && (
                      <span className="text-[10px] font-mono px-1.5 py-0.5 rounded-md font-medium"
                        style={{ backgroundColor: `${color}18`, color }}>
                        v{m.active_version} active
                      </span>
                    )}
                  </div>
                  <div className="text-xs text-muted-foreground mt-0.5">
                    {installedCount} {t('common.installed')} · {results.length || '?'} {t('common.availableCount')}
                  </div>
                </div>
                <div className="text-muted-foreground">
                  {open ? <ChevronDownIcon className="w-4 h-4" /> : <ChevronRightIcon className="w-4 h-4" />}
                </div>
              </button>

              {/* Version list */}
              {open && (
                <div className="border-t border-border">
                  {/* Installed versions */}
                  {m.versions.map((ver, idx) => {
                    const isActive = m.active_version === ver;
                    return (
                      <div key={ver}
                        className={`flex items-center gap-3 px-5 py-3 border-b border-border/50 last:border-b-0 transition-colors ${
                          isActive ? 'bg-success/5' : 'hover:bg-muted/20'
                        }`}
                        style={{ animationDelay: `${idx * 30}ms` }}
                      >
                        <div className="w-4 flex items-center justify-center shrink-0">
                          {isActive ? <CheckCircleIcon className="w-4 h-4 text-success" /> : <PackageIcon className="w-3.5 h-3.5 text-muted-foreground" />}
                        </div>
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2">
                            <span className={`font-mono text-sm font-medium ${isActive ? 'text-success' : 'text-foreground'}`}>
                              {m.name}@{ver}
                            </span>
                            {m.version_labels?.[idx] && (
                              <span className="text-[11px] font-mono text-muted-foreground/60">{m.version_labels[idx]}</span>
                            )}
                            {isActive && (
                              <span className="text-[10px] px-1.5 py-0.5 rounded bg-success/15 text-success border border-success/25">{t('common.active')}</span>
                            )}
                          </div>
                          <div className="text-[11px] text-muted-foreground font-mono mt-0.5 truncate">
                            {m.source_paths?.[idx] || `~/.envswitch/envs/${m.name}/${ver}`}
                          </div>
                        </div>
                        <div className="flex items-center gap-1.5 shrink-0">
                          {isActive ? (
                            <button onClick={() => uncover(m.name)}
                              className="flex items-center gap-1 px-2.5 py-1.5 rounded-md text-xs bg-warning/10 text-warning hover:bg-warning/20 border border-warning/30 font-medium"
                            ><XCircleIcon className="w-3 h-3" /> {t('common.uncover')}</button>
                          ) : (
                            <button onClick={() => cover(m.name, ver)}
                              className="flex items-center gap-1 px-2.5 py-1.5 rounded-md text-xs bg-success/10 text-success hover:bg-success/20 border border-success/30 font-medium"
                            ><PlayIcon className="w-3 h-3" /> {t('common.cover')}</button>
                          )}
                          {/* Only show uninstall for modules that can be removed */}
                          {(m.uninstallable?.[idx] !== false) && (
                            <button onClick={() => doUninstall(m.name, ver)}
                              className="flex items-center justify-center w-7 h-7 rounded-md text-xs text-muted-foreground hover:bg-destructive/10 hover:text-destructive border border-transparent hover:border-destructive/30"
                            ><Trash2Icon className="w-3.5 h-3.5" /></button>
                          )}
                        </div>
                      </div>
                    );
                  })}

                  {/* Available — explicitly fetched by user */}
                  <div className="border-t border-border/50">
                    <div className="flex items-center gap-2 px-5 py-3 bg-muted/5 border-b border-border/50">
                      <CircleIcon className="w-3 h-3 text-muted-foreground" />
                      <span className="text-[10px] text-muted-foreground uppercase tracking-wider">{t('common.available')}</span>
                      <div className="flex-1" />
                      {results.length > 0 && <span className="text-[10px] text-muted-foreground font-mono">{results.length} found</span>}
                      <button onClick={() => fetchAvailable(m.name)}
                        className={`flex items-center gap-1 px-2.5 py-1 rounded-md text-[11px] font-medium transition-all border ${
                          isSearching ? 'bg-accent/20 border-accent/30 text-accent-foreground' : 'bg-accent/50 text-accent-foreground hover:bg-accent border-border/50'
                        }`}
                      >
                        {isSearching ? <Loader2Icon className="w-2.5 h-2.5 animate-spin" /> : <DownloadIcon className="w-2.5 h-2.5" />}
                        {t('common.fetch')}
                      </button>
                    </div>
                    {results.length > 0 && (
                      <div className="max-h-64 overflow-y-auto">
                        {results.slice(0, 300).map(ver => {
                          // Skip if exact match or if installed version starts with this (8.0 is covered by 8.0.46)
                          if (installedSet.has(ver)) return null;
                          if (m.versions.some(iv => iv.startsWith(ver + '.') || iv === ver)) return null;
                          const busy = installing === `${m.name}:${ver}`;
                          return (
                            <div key={ver} className="flex items-center gap-3 px-5 py-2.5 border-b border-border/50 last:border-b-0 hover:bg-muted/20">
                              <div className="flex-1 min-w-0">
                                <span className="font-mono text-sm text-muted-foreground">{m.name}@{ver}</span>
                              </div>
                              <button onClick={() => doInstall(m.name, ver)} disabled={busy}
                                className="flex items-center gap-1 px-2.5 py-1.5 rounded-md text-xs bg-accent/50 text-accent-foreground hover:bg-accent border border-border/50 font-medium disabled:opacity-50"
                              >
                                {busy ? <Loader2Icon className="w-2.5 h-2.5 animate-spin" /> : <DownloadIcon className="w-2.5 h-2.5" />}
                                {t('common.install')}
                              </button>
                            </div>
                          );
                        })}
                      </div>
                    )}
                    {!isSearching && results.length === 0 && (
                      <div className="px-5 py-3 text-[11px] text-muted-foreground/50 font-mono">
{t('versions.clickFetch')}
                      </div>
                    )}
                  </div>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
