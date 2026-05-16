import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';
import TopBar from '../components/TopBar';
import { PlusIcon, XIcon, SearchIcon, DownloadIcon, Trash2Icon, Loader2Icon } from 'lucide-react';

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
  const [searchMod, setSearchMod] = useState<string | null>(null);
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
    try {
      await invoke('cover_module', { module: mod, version: ver, global: false });
      toast.success(`${mod} ${ver} ${t('toast.covered')}`);
      refresh();
    } catch (e) { toast.error(`${e}`); }
  };
  const uncover = async (mod: string) => {
    try {
      await invoke('uncover_module', { module: mod });
      toast.success(`${mod} ${t('toast.uncovered')}`);
      refresh();
    } catch (e) { toast.error(`${e}`); }
  };
  const linkCustom = async () => {
    try {
      await invoke('link_module', { module: addMod, version: addVer, path: addPath });
      toast.success(`${addMod} ${addVer} linked`);
      setShowAdd(false); setAddMod(''); setAddVer(''); setAddPath('');
      refresh();
    } catch (e) { toast.error(`${e}`); }
  };

  const doSearch = async (mod: string) => {
    if (searchMod === mod) { setSearchMod(null); setSearchResults([]); return; }
    setSearchMod(mod); setSearching(true);
    try {
      const results = await invoke<string[]>('search_versions', { module: mod });
      setSearchResults(results.slice(0, 50)); // show top 50
    } catch (e) { toast.error(`${e}`); }
    setSearching(false);
  };

  const doInstall = async (mod: string, ver: string) => {
    setInstalling(`${mod}:${ver}`);
    try {
      await invoke('install_version', { module: mod, version: ver });
      toast.success(`${mod} ${ver} installed`);
      refresh();
    } catch (e) { toast.error(`${e}`); }
    setInstalling(null);
  };

  const doUninstall = async (mod: string, ver: string) => {
    try {
      await invoke('uninstall_version', { module: mod, version: ver });
      toast.success(`${mod} ${ver} uninstalled`);
      refresh();
    } catch (e) { toast.error(`${e}`); }
  };

  return (
    <div className="flex flex-col h-full">
      <TopBar title={t('nav.versions')} subtitle="Cover / Uncover / Search / Install" onRefresh={refresh} />

      <div className="flex-1 overflow-y-auto p-4">
        {/* Add Custom */}
        <div className="mb-4">
          {!showAdd ? (
            <button onClick={() => setShowAdd(true)}
              className="flex items-center gap-1.5 px-3 py-1.5 text-xs rounded-md border border-dashed border-primary/40 text-primary hover:bg-primary/10 transition-colors"
            >
              <PlusIcon className="w-3.5 h-3.5" /> Add custom path
            </button>
          ) : (
            <div className="rounded-xl border border-primary/30 bg-card p-4 space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">Add Custom Environment</span>
                <button onClick={() => setShowAdd(false)}><XIcon className="w-4 h-4 text-muted-foreground" /></button>
              </div>
              <div className="grid grid-cols-3 gap-2">
                <input value={addMod} onChange={e => setAddMod(e.target.value)} placeholder="module (e.g. jdk)" className="px-3 py-2 rounded-md border border-border bg-background text-sm text-foreground placeholder:text-muted-foreground" />
                <input value={addVer} onChange={e => setAddVer(e.target.value)} placeholder="version (e.g. 21)" className="px-3 py-2 rounded-md border border-border bg-background text-sm text-foreground placeholder:text-muted-foreground" />
                <input value={addPath} onChange={e => setAddPath(e.target.value)} placeholder="/path/to/install" className="px-3 py-2 rounded-md border border-border bg-background text-sm text-foreground placeholder:text-muted-foreground" />
              </div>
              <button onClick={linkCustom} disabled={!addMod || !addVer || !addPath}
                className="px-4 py-1.5 text-xs rounded-md bg-primary text-primary-foreground font-medium hover:bg-primary/90 disabled:opacity-40"
              >Link</button>
            </div>
          )}
        </div>

        {/* Module cards */}
        <div className="grid grid-cols-2 gap-4">
          {modules.map((m) => (
            <div key={m.name} className="rounded-xl border border-border bg-card overflow-hidden">
              <div className="flex items-center gap-2 px-4 py-3 border-b border-border">
                <span className="text-xs px-2 py-0.5 rounded bg-primary/10 text-primary font-mono">{m.name}</span>
                <span className="font-medium text-sm">{m.display_name}</span>
                <span className="text-xs text-muted-foreground">{m.category}</span>
                <div className="flex-1" />
                <button onClick={() => doSearch(m.name)}
                  className={`flex items-center gap-1 px-2 py-1 text-[11px] rounded transition-colors ${
                    searchMod === m.name ? 'bg-primary/20 text-primary' : 'text-muted-foreground hover:text-foreground'
                  }`}
                ><SearchIcon className="w-3 h-3" /> Search</button>
              </div>

              <div className="p-4 space-y-2">
                {/* Search results */}
                {searchMod === m.name && (
                  <div className="mb-3 p-3 rounded-lg bg-muted/30 border border-border max-h-48 overflow-y-auto">
                    {searching ? (
                      <div className="flex items-center gap-2 text-xs text-muted-foreground"><Loader2Icon className="w-3 h-3 animate-spin" /> Searching...</div>
                    ) : (
                      <div className="flex flex-wrap gap-1">
                        {searchResults.map(ver => {
                          const installed = m.versions.includes(ver);
                          const busy = installing === `${m.name}:${ver}`;
                          return (
                            <div key={ver} className={`flex items-center gap-1 px-2 py-1 rounded text-[11px] font-mono ${
                              installed ? 'bg-primary/10 text-primary' : 'bg-background border border-border'
                            }`}>
                              {ver}
                              {!installed && (
                                <button onClick={() => doInstall(m.name, ver)} disabled={busy}
                                  className="ml-1 p-0.5 rounded hover:bg-primary/20 text-muted-foreground hover:text-primary"
                                >{busy ? <Loader2Icon className="w-2.5 h-2.5 animate-spin" /> : <DownloadIcon className="w-2.5 h-2.5" />}</button>
                              )}
                            </div>
                          );
                        })}
                      </div>
                    )}
                  </div>
                )}

                {/* Installed versions */}
                {m.versions.length === 0 && !searchMod && (
                  <p className="text-xs text-muted-foreground">{t('common.noVersions')}</p>
                )}
                {m.versions.map((ver) => {
                  const isActive = m.active_version === ver;
                  return (
                    <div key={ver} className={`flex items-center justify-between px-3 py-2 rounded-md text-xs border ${
                      isActive ? 'border-primary/40 bg-primary/5' : 'border-border'
                    }`}>
                      <span className="font-mono">{ver} {isActive && <span className="text-primary ml-1">✓</span>}</span>
                      <div className="flex gap-1">
                        <button onClick={() => cover(m.name, ver)} disabled={isActive}
                          className="px-2 py-1 rounded bg-primary/20 hover:bg-primary/30 text-primary disabled:opacity-30 text-[11px]"
                        >{t('common.cover')}</button>
                        {isActive && (
                          <button onClick={() => uncover(m.name)}
                            className="px-2 py-1 rounded bg-secondary hover:bg-secondary/80 text-[11px]"
                          >{t('common.uncover')}</button>
                        )}
                        <button onClick={() => doUninstall(m.name, ver)}
                          className="px-2 py-1 rounded hover:bg-red-500/15 text-muted-foreground hover:text-red-400 text-[11px]"
                        ><Trash2Icon className="w-3 h-3" /></button>
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
