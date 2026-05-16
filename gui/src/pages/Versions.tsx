import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';
import TopBar from '../components/TopBar';
import { PlusIcon, XIcon, DownloadIcon, Trash2Icon, Loader2Icon, ChevronDownIcon, ChevronRightIcon } from 'lucide-react';

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
  const [searchResults, setSearchResults] = useState<string[]>([]);
  const [searching, setSearching] = useState(false);
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
    try { await invoke('link_module', { module: addMod, version: addVer, path: addPath }); toast.success('linked'); setShowAdd(false); refresh(); }
    catch (e) { toast.error(`${e}`); }
  };

  const toggleExpand = async (mod: string) => {
    if (expandedMod === mod) { setExpandedMod(null); setSearchResults([]); return; }
    setExpandedMod(mod); setSearching(true);
    try { setSearchResults(await invoke<string[]>('search_versions', { module: mod })); }
    catch { setSearchResults([]); }
    setSearching(false);
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

        {/* Module list — one row per module, expandable */}
        <div className="px-4 pb-4">
          {modules.map((m) => {
            const expanded = expandedMod === m.name;
            const installedSet = new Set(m.versions);
            return (
              <div key={m.name} className="border-b border-border last:border-b-0">
                {/* Module header row */}
                <div
                  className="flex items-center gap-3 px-3 py-3 hover:bg-muted/30 cursor-pointer transition-colors"
                  onClick={() => toggleExpand(m.name)}
                >
                  {expanded ? <ChevronDownIcon className="w-4 h-4 text-muted-foreground shrink-0" /> : <ChevronRightIcon className="w-4 h-4 text-muted-foreground shrink-0" />}
                  <span className="text-xs px-1.5 py-0.5 rounded bg-primary/10 text-primary font-mono shrink-0">{m.name}</span>
                  <span className="font-medium text-sm">{m.display_name}</span>
                  <span className="text-[10px] text-muted-foreground uppercase">{m.category}</span>
                  <div className="flex-1" />
                  {/* Installed versions shown inline */}
                  {m.versions.length > 0 ? (
                    <div className="flex items-center gap-1">
                      {m.versions.map(ver => {
                        const isActive = m.active_version === ver;
                        return (
                          <div key={ver} className={`flex items-center gap-1 px-2 py-1 rounded text-[11px] font-mono ${
                            isActive ? 'bg-primary/15 text-primary ring-1 ring-primary/30' : 'bg-muted/50 text-foreground/70'
                          }`}>
                            {ver}
                            {isActive && <span className="text-primary font-bold text-[10px]">✓</span>}
                          </div>
                        );
                      })}
                    </div>
                  ) : (
                    <span className="text-[11px] text-muted-foreground italic">none installed</span>
                  )}
                  <span className="text-[10px] text-muted-foreground ml-2">{m.versions.length} ver</span>
                </div>

                {/* Expanded: installed versions details + search */}
                {expanded && (
                  <div className="px-8 pb-4 space-y-3">
                    {/* Installed versions — one row each */}
                    {m.versions.length > 0 && (
                      <div className="space-y-1.5">
                        <p className="text-[10px] text-muted-foreground uppercase tracking-wider">Installed</p>
                        {m.versions.map(ver => {
                          const isActive = m.active_version === ver;
                          return (
                            <div key={ver} className={`flex items-center gap-3 px-3 py-2 rounded-md text-xs border ${
                              isActive ? 'border-primary/40 bg-primary/5' : 'border-border'
                            }`}>
                              <span className="font-mono text-sm w-32 shrink-0">{ver}</span>
                              {isActive ? (
                                <button onClick={(e) => { e.stopPropagation(); uncover(m.name); }}
                                  className="px-2.5 py-1 rounded bg-secondary hover:bg-secondary/80 text-[11px]"
                                >{t('common.uncover')}</button>
                              ) : (
                                <button onClick={(e) => { e.stopPropagation(); cover(m.name, ver); }}
                                  className="px-2.5 py-1 rounded bg-primary/20 hover:bg-primary/30 text-primary text-[11px]"
                                >{t('common.cover')}</button>
                              )}
                              <button onClick={(e) => { e.stopPropagation(); doUninstall(m.name, ver); }}
                                className="px-2 py-1 rounded hover:bg-red-500/15 text-muted-foreground hover:text-red-400 text-[11px] ml-auto"
                              ><Trash2Icon className="w-3 h-3" /></button>
                            </div>
                          );
                        })}
                      </div>
                    )}

                    {/* Search results */}
                    <div>
                      <p className="text-[10px] text-muted-foreground uppercase tracking-wider mb-2">Available ({searchResults.length})</p>
                      {searching ? (
                        <div className="flex items-center gap-2 text-xs text-muted-foreground"><Loader2Icon className="w-3 h-3 animate-spin" /> Searching...</div>
                      ) : (
                        <div className="flex flex-wrap gap-1 max-h-48 overflow-y-auto">
                          {searchResults.slice(0, 60).map(ver => {
                            const installed = installedSet.has(ver);
                            const busy = installing === `${m.name}:${ver}`;
                            return (
                              <div key={ver} className={`flex items-center gap-1 px-2 py-1 rounded text-[11px] font-mono ${
                                installed ? 'bg-primary/10 text-primary opacity-50' : 'bg-muted/30 border border-border'
                              }`}>
                                {ver}
                                {!installed && (
                                  <button onClick={(e) => { e.stopPropagation(); doInstall(m.name, ver); }} disabled={busy}
                                    className="ml-1 p-0.5 rounded hover:bg-primary/20 text-muted-foreground hover:text-primary"
                                  >{busy ? <Loader2Icon className="w-2.5 h-2.5 animate-spin" /> : <DownloadIcon className="w-2.5 h-2.5" />}</button>
                                )}
                              </div>
                            );
                          })}
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
    </div>
  );
}
