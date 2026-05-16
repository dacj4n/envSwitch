import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';
import TopBar from '../components/TopBar';
import { PlusIcon, XIcon, DownloadIcon, Trash2Icon, Loader2Icon, ChevronDownIcon, ChevronRightIcon, FolderIcon } from 'lucide-react';

interface ModuleInfo {
  name: string; display_name: string; category: string;
  versions: string[]; active_version: string | null;
}

export default function VersionsPage() {
  const { t } = useTranslation();
  const [modules, setModules] = useState<ModuleInfo[]>([]);
  const [showAdd, setShowAdd] = useState(false);
  const [addMod, setAddMod] = useState('');
  const [addVer, setAddVer] = useState('');
  const [addPath, setAddPath] = useState('');
  const [expandedMod, setExpandedMod] = useState<string | null>(null);
  const [searchResults, setSearchResults] = useState<Record<string, string[]>>({});
  const [searching, setSearching] = useState<Record<string, boolean>>({});
  const [installing, setInstalling] = useState<string | null>(null);

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
  const linkCustom = async () => {
    try { await invoke('link_module', { module: addMod, version: addVer, path: addPath }); toast.success('linked'); setShowAdd(false); setAddMod(''); setAddVer(''); setAddPath(''); refresh(); }
    catch (e) { toast.error(`${e}`); }
  };

  const toggleExpand = async (mod: string) => {
    if (expandedMod === mod) { setExpandedMod(null); return; }
    setExpandedMod(mod);
    if (searchResults[mod]) return; // already searched
    setSearching(prev => ({ ...prev, [mod]: true }));
    try {
      const results = await invoke<string[]>('search_versions', { module: mod });
      setSearchResults(prev => ({ ...prev, [mod]: results }));
    } catch { setSearchResults(prev => ({ ...prev, [mod]: [] })); }
    setSearching(prev => ({ ...prev, [mod]: false }));
  };

  const doInstall = async (mod: string, ver: string) => {
    setInstalling(`${mod}:${ver}`);
    try { await invoke('install_version', { module: mod, version: ver }); toast.success(`${mod} ${ver} installed`); refresh(); }
    catch (e) { toast.error(`${e}`); }
    setInstalling(null);
  };
  const doUninstall = async (mod: string, ver: string) => {
    try { await invoke('uninstall_version', { module: mod, version: ver }); toast.success('uninstalled'); refresh(); }
    catch (e) { toast.error(`${e}`); }
  };

  const homeDir = '~/.envswitch/envs';

  return (
    <div className="flex flex-col h-full">
      <TopBar title={t('nav.versions')} subtitle={`${modules.length} modules`} onRefresh={refresh} />

      <div className="flex-1 overflow-y-auto">
        {/* Add custom */}
        <div className="px-4 pt-3">
          {!showAdd ? (
            <button onClick={() => setShowAdd(true)} className="flex items-center gap-1.5 px-3 py-1.5 text-xs rounded-md border border-dashed border-primary/40 text-primary hover:bg-primary/10">
              <PlusIcon className="w-3.5 h-3.5" /> Add custom path
            </button>
          ) : (
            <div className="rounded-lg border border-primary/30 bg-card p-3 mb-3 flex items-center gap-2">
              <input value={addMod} onChange={e => setAddMod(e.target.value)} placeholder="module" className="w-20 px-2 py-1.5 rounded border border-border bg-background text-xs" />
              <input value={addVer} onChange={e => setAddVer(e.target.value)} placeholder="version" className="w-20 px-2 py-1.5 rounded border border-border bg-background text-xs" />
              <input value={addPath} onChange={e => setAddPath(e.target.value)} placeholder="/path/to/install" className="flex-1 px-2 py-1.5 rounded border border-border bg-background text-xs" />
              <button onClick={linkCustom} disabled={!addMod||!addVer||!addPath} className="px-3 py-1.5 text-xs rounded bg-primary text-primary-foreground disabled:opacity-40">Link</button>
              <button onClick={() => setShowAdd(false)}><XIcon className="w-4 h-4 text-muted-foreground" /></button>
            </div>
          )}
        </div>

        {/* Module list */}
        <div className="px-4 pb-4 space-y-0">
          {modules.map((m) => {
            const expanded = expandedMod === m.name;
            const results = searchResults[m.name] || [];
            const isSearching = searching[m.name] || false;
            const installedSet = new Set(m.versions);

            return (
              <div key={m.name} className="border-b border-border last:border-b-0">
                {/* Header */}
                <div className="flex items-center gap-3 px-3 py-3 hover:bg-muted/30 cursor-pointer transition-colors" onClick={() => toggleExpand(m.name)}>
                  {expanded ? <ChevronDownIcon className="w-4 h-4 text-muted-foreground shrink-0" /> : <ChevronRightIcon className="w-4 h-4 text-muted-foreground shrink-0" />}
                  <span className="text-xs px-1.5 py-0.5 rounded bg-primary/10 text-primary font-mono shrink-0">{m.name}</span>
                  <span className="font-medium text-sm">{m.display_name}</span>
                  <span className="text-[10px] text-muted-foreground uppercase">{m.category}</span>
                  <div className="flex-1" />
                  {m.versions.length > 0 ? (
                    <div className="flex items-center gap-1">
                      {m.versions.slice(0, 4).map(ver => (
                        <span key={ver} className={`text-[11px] font-mono px-1.5 py-0.5 rounded ${m.active_version === ver ? 'bg-primary/15 text-primary' : 'text-foreground/50'}`}>{ver}{m.active_version === ver ? ' ✓' : ''}</span>
                      ))}
                      {m.versions.length > 4 && <span className="text-[10px] text-muted-foreground">+{m.versions.length - 4}</span>}
                    </div>
                  ) : (
                    <span className="text-[11px] text-muted-foreground italic">none</span>
                  )}
                </div>

                {/* Expanded */}
                {expanded && (
                  <div className="px-0 pb-4">
                    {/* Installed versions — always show with path */}
                    {m.versions.length > 0 && (
                      <div>
                        {m.versions.map(ver => {
                          const isActive = m.active_version === ver;
                          return (
                            <div key={ver}
                              className={`flex items-center gap-3 mx-8 px-3 py-2 text-xs border-b border-border/50 transition-colors ${
                                isActive ? 'bg-primary/5' : 'hover:bg-muted/20'
                              }`}
                            >
                              <FolderIcon className="w-3 h-3 text-muted-foreground shrink-0" />
                              <span className="font-mono text-xs w-24 shrink-0">{ver}</span>
                              <span className="text-[10px] text-muted-foreground truncate">{homeDir}/{m.name}/{ver}</span>
                              {isActive && <span className="text-[10px] text-primary font-medium ml-auto mr-2">active</span>}
                              <div className="flex items-center gap-1 ml-auto shrink-0">
                                {isActive ? (
                                  <button onClick={(e) => { e.stopPropagation(); uncover(m.name); }}
                                    className="px-2.5 py-1 rounded bg-secondary hover:bg-secondary/80 text-[11px]"
                                  >Uncover</button>
                                ) : (
                                  <button onClick={(e) => { e.stopPropagation(); cover(m.name, ver); }}
                                    className="px-2.5 py-1 rounded bg-primary/20 hover:bg-primary/30 text-primary text-[11px]"
                                  >Cover</button>
                                )}
                                <button onClick={(e) => { e.stopPropagation(); doUninstall(m.name, ver); }}
                                  className="px-2 py-1 rounded hover:bg-red-500/15 text-muted-foreground hover:text-red-400"
                                ><Trash2Icon className="w-3 h-3" /></button>
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    )}

                    {/* Search results — only when expanded, same row format */}
                    {isSearching ? (
                      <div className="flex items-center gap-2 mx-8 mt-3 text-xs text-muted-foreground">
                        <Loader2Icon className="w-3 h-3 animate-spin" /> Searching available versions...
                      </div>
                    ) : results.length > 0 && (
                      <div>
                        <div className="mx-8 mt-3 mb-1 text-[10px] text-muted-foreground uppercase tracking-wider">Available ({results.length})</div>
                        <div className="max-h-64 overflow-y-auto">
                          {results.slice(0, 80).map(ver => {
                            const installed = installedSet.has(ver);
                            const busy = installing === `${m.name}:${ver}`;
                            if (installed) return null;
                            return (
                              <div key={ver} className="flex items-center gap-3 mx-8 px-3 py-2 text-xs border-b border-border/50 hover:bg-muted/20">
                                <span className="font-mono text-xs w-24 shrink-0">{ver}</span>
                                <span className="text-[10px] text-muted-foreground">remote</span>
                                <div className="flex items-center gap-1 ml-auto shrink-0">
                                  <button onClick={(e) => { e.stopPropagation(); doInstall(m.name, ver); }} disabled={busy}
                                    className="flex items-center gap-1 px-2.5 py-1 rounded bg-primary/20 hover:bg-primary/30 text-primary text-[11px] disabled:opacity-50"
                                  >
                                    {busy ? <Loader2Icon className="w-3 h-3 animate-spin" /> : <DownloadIcon className="w-3 h-3" />}
                                    Install
                                  </button>
                                </div>
                              </div>
                            );
                          })}
                        </div>
                      </div>
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
