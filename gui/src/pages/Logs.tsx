import { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import TopBar from '../components/TopBar';
import { ScrollTextIcon, InfoIcon, CheckCircle2Icon, AlertTriangleIcon, XCircleIcon, Loader2Icon } from 'lucide-react';

const LEVEL_COLORS: Record<string, string> = {
  ALL: '#60a5fa', INFO: '#60a5fa', SUCCESS: '#22c55e', WARN: '#f59e0b', ERROR: '#ef4444',
};
const LEVEL_ICONS: Record<string, React.ComponentType<any>> = {
  ALL: InfoIcon, INFO: InfoIcon, SUCCESS: CheckCircle2Icon, WARN: AlertTriangleIcon, ERROR: XCircleIcon,
};

interface ServiceInfo { name: string; status: string; pid: number | null; port: number | null; }

function parseLevel(line: string): string {
  if (/\[ERROR\]|error:|Error:/i.test(line)) return 'ERROR';
  if (/\[WARNING\]|\[WARN\]|warning:|warn:/i.test(line)) return 'WARN';
  if (/\[System\]|\[Note\]|started|ready|initialized/i.test(line)) return 'SUCCESS';
  return 'INFO';
}

export default function LogsPage() {
  const { t } = useTranslation();
  const [services, setServices] = useState<ServiceInfo[]>([]);
  const [service, setService] = useState('mysql');
  const [version, setVersion] = useState('');
  const [filter, setFilter] = useState('ALL');
  const [lines, setLines] = useState(100);
  const [logs, setLogs] = useState<{ text: string; level: string }[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    invoke<ServiceInfo[]>('get_services').then(s => {
      setServices(s);
      if (s.length > 0) setService(s[0].name);
    });
  }, []);

  useEffect(() => {
    invoke<string[]>('list_installed_versions', { module: service }).then(v => {
      if (v.length > 0) setVersion(v[0]);
    }).catch(() => {});
  }, [service]);

  const refresh = useCallback(async () => {
    if (!version) return;
    setLoading(true);
    try {
      const raw = await invoke<string[]>('read_service_logs', { module: service, version, lines });
      setLogs(raw.map(text => ({ text, level: parseLevel(text) })));
    } catch { setLogs([]); }
    setLoading(false);
  }, [service, version, lines]);

  useEffect(() => { if (version) refresh(); }, [version]);

  return (
    <div className="flex flex-col h-full min-h-0">
      <TopBar title={t('nav.logs')} subtitle={t('service.logsSubtitle')} />
      <div className="flex-1 overflow-y-auto px-5 py-4 space-y-4">
        <div className="rounded-xl border border-border bg-card overflow-hidden">
          <div className="flex items-center gap-3 px-5 py-3">
            <ScrollTextIcon className="w-4 h-4 text-primary" />
            <span className="text-sm font-semibold text-foreground">{t('service.logViewer')}</span>
            <div className="flex items-center gap-1 ml-auto">
              <select value={service} onChange={e => setService(e.target.value)}
                className="px-2.5 py-1.5 rounded-md border border-border bg-background text-xs text-foreground">
                {services.map(s => <option key={s.name} value={s.name}>{s.name}</option>)}
                {services.length === 0 && <><option value="mysql">mysql</option><option value="pgsql">pgsql</option></>}
              </select>
              <button onClick={refresh} disabled={loading}
                className="px-3 py-1.5 rounded-md text-xs bg-secondary hover:bg-accent border border-border text-secondary-foreground flex items-center gap-1"
              >
                {loading ? <Loader2Icon className="w-3 h-3 animate-spin" /> : null}
                {t('common.refresh')}
              </button>
            </div>
          </div>

          {/* Level filters */}
          <div className="flex items-center gap-1 px-5 py-2 border-t border-border/50 bg-muted/5">
            {['ALL', 'INFO', 'SUCCESS', 'WARN', 'ERROR'].map(level => {
              const color = LEVEL_COLORS[level];
              const Icon = LEVEL_ICONS[level];
              return (
                <button key={level} onClick={() => setFilter(level)}
                  className={`flex items-center gap-1 px-2.5 py-1 rounded-md text-[11px] font-medium transition-all border ${
                    filter === level ? 'border-current' : 'border-transparent hover:border-border'
                  }`}
                  style={{ color, backgroundColor: filter === level ? `${color}15` : 'transparent' }}
                >
                  <Icon className="w-3 h-3" /> {level}
                </button>
              );
            })}
          </div>

          {/* Log lines */}
          <div className="divide-y divide-border/30 bg-background font-mono text-xs max-h-[calc(100vh-320px)] overflow-y-auto">
            {logs
              .filter(l => filter === 'ALL' || l.level === filter)
              .map((line, i) => {
                const color = LEVEL_COLORS[line.level] || LEVEL_COLORS.INFO;
                const Icon = LEVEL_ICONS[line.level] || InfoIcon;
                return (
                  <div key={i} className="flex items-start gap-2 px-4 py-2 hover:bg-muted/10">
                    <Icon className="w-3.5 h-3.5 mt-0.5 shrink-0" style={{ color }} />
                    <span style={{ color }} className="whitespace-pre-wrap break-all">{line.text}</span>
                  </div>
                );
              })}
            {logs.length === 0 && !loading && (
              <div className="px-4 py-12 text-center text-muted-foreground text-xs">
                {version ? t('common.noLogs') : t('common.noVersionFound')}
              </div>
            )}
            {loading && (
              <div className="px-4 py-12 text-center text-muted-foreground text-xs flex items-center justify-center gap-2">
                <Loader2Icon className="w-3 h-3 animate-spin" /> {t('common.loading')}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
