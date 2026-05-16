import { useEffect, useState } from 'react';
import { Link, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { LayersIcon, DatabaseIcon, ActivityIcon, ScrollTextIcon, HeartPulseIcon, SettingsIcon, ZapIcon, Loader2Icon, CheckCircleIcon, XCircleIcon, ExternalLinkIcon } from 'lucide-react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

interface JobEntry {
  id: string; kind: string; module: string; version: string;
  status: string; progress: number; message: string;
}

export default function Sidebar() {
  const { t } = useTranslation();
  const location = useLocation();
  const [platform, setPlatform] = useState('');
  const [jobs, setJobs] = useState<JobEntry[]>([]);
  const [showJobs, setShowJobs] = useState(false);

  useEffect(() => {
    invoke<string>('get_platform').then(setPlatform).catch(() => {});
    const unlisten = listen<JobEntry>('job-update', (ev) => {
      const j = ev.payload;
      setJobs(prev => {
        const existing = prev.findIndex(x => x.id === j.id);
        if (existing >= 0) {
          const updated = [...prev];
          updated[existing] = j;
          return updated.filter(x => x.status === 'running' || updated.indexOf(x) === updated.length - 1 || updated.indexOf(x) > updated.length - 6);
        }
        return [j, ...prev].slice(0, 10);
      });
    });
    return () => { unlisten.then(fn => fn()); };
  }, []);

  const activeJobs = jobs.filter(j => j.status === 'running');
  const completedJobs = jobs.filter(j => j.status !== 'running');

  return (
    <aside className="flex flex-col w-[200px] min-w-[200px] h-screen bg-sidebar border-r border-sidebar-border">
      <div className="flex items-center gap-2.5 px-5 py-5 border-b border-sidebar-border">
        <div className="flex items-center justify-center w-8 h-8 rounded-lg gradient-brand">
          <ZapIcon className="w-4 h-4 text-white" />
        </div>
        <div>
          <div className="text-foreground font-semibold text-sm">{t('app.name')}</div>
          <div className="text-muted-foreground text-[10px] font-mono">{t('app.version')} · {platform}</div>
        </div>
      </div>

      <nav className="flex-1 px-3 py-4 flex flex-col gap-0.5">
        {[
          { path: '/', label: t('nav.versions'), icon: LayersIcon },
          { path: '/services', label: t('nav.services'), icon: DatabaseIcon },
          { path: '/status', label: t('nav.status'), icon: ActivityIcon },
          { path: '/logs', label: t('nav.logs'), icon: ScrollTextIcon },
          { path: '/doctor', label: t('nav.doctor'), icon: HeartPulseIcon },
        ].map(({ path, label, icon: Icon }) => {
          const active = location.pathname === path;
          return (
            <Link key={path} to={path}
              className={`flex items-center gap-3 px-3 py-2.5 rounded-md text-sm transition-all ${
                active ? 'bg-sidebar-accent text-foreground font-medium' : 'text-sidebar-foreground hover:bg-sidebar-accent/60'
              }`}
            >
              <Icon className={`w-4 h-4 ${active ? 'text-primary' : 'text-muted-foreground'}`} />
              {label}
            </Link>
          );
        })}
      </nav>

      <div className="px-3 py-4 border-t border-sidebar-border space-y-1">
        <Link to="/settings"
          className={`flex items-center gap-3 px-3 py-2.5 rounded-md text-sm transition-all ${
            location.pathname === '/settings' ? 'bg-sidebar-accent text-foreground font-medium' : 'text-sidebar-foreground hover:bg-sidebar-accent/60'
          }`}
        >
          <SettingsIcon className="w-4 h-4 text-muted-foreground" />
          {t('nav.settings')}
        </Link>

        {/* Job Center */}
        {activeJobs.length > 0 && (
          <button onClick={() => setShowJobs(!showJobs)}
            className="w-full mt-1 px-3 py-2 rounded-md bg-sidebar-accent/50 flex items-center gap-2 hover:bg-sidebar-accent/80 transition-colors"
          >
            <Loader2Icon className="w-3.5 h-3.5 text-primary animate-spin" />
            <span className="text-xs text-sidebar-foreground">{activeJobs.length} Job{activeJobs.length > 1 ? 's' : ''}</span>
          </button>
        )}

        {/* Jobs dropdown */}
        {showJobs && (
          <div className="space-y-1 max-h-64 overflow-y-auto">
            {[...activeJobs, ...completedJobs.slice(0, 3)].map(j => (
              <div key={j.id} className="px-2 py-1.5 rounded border border-border/50 bg-sidebar/80">
                <div className="flex items-center gap-1.5">
                  {j.status === 'running' && <Loader2Icon className="w-3 h-3 text-primary animate-spin shrink-0" />}
                  {j.status === 'success' && <CheckCircleIcon className="w-3 h-3 text-success shrink-0" />}
                  {j.status === 'failed' && <XCircleIcon className="w-3 h-3 text-destructive shrink-0" />}
                  <span className="text-[10px] font-mono text-sidebar-foreground truncate">{j.module} {j.version}</span>
                </div>
                <div className="flex items-center justify-between mt-0.5">
                  <span className="text-[9px] text-muted-foreground truncate">{j.message}</span>
                  <button onClick={() => window.open(`/install/${j.id}`, '_blank')}
                    className="p-0.5 rounded hover:bg-accent/50 text-muted-foreground" title="Open install window">
                    <ExternalLinkIcon className="w-2.5 h-2.5" />
                  </button>
                </div>
                {j.status === 'running' && (
                  <div className="mt-1 h-1 rounded-full bg-muted/30">
                    <div className="h-full rounded-full bg-primary transition-all" style={{ width: `${Math.max(2, j.progress * 100)}%` }} />
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </aside>
  );
}
