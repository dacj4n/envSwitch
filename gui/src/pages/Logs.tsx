import { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import TopBar from '../components/TopBar';
import { ScrollTextIcon, InfoIcon, CheckCircle2Icon, AlertTriangleIcon, XCircleIcon, Loader2Icon } from 'lucide-react';

const LEVEL_COLORS: Record<string, string> = {
  ALL: '#60a5fa', OK: '#22c55e', INFO: '#60a5fa', WARN: '#f59e0b', ERR: '#ef4444',
};
const LEVEL_ICONS: Record<string, React.ComponentType<any>> = {
  ALL: InfoIcon, OK: CheckCircle2Icon, INFO: InfoIcon, WARN: AlertTriangleIcon, ERR: XCircleIcon,
};

function parseLevelLine(line: string): string {
  if (/\bERR\b/.test(line)) return 'ERR';
  if (/\bWARN\b/.test(line)) return 'WARN';
  if (/\bOK\b/.test(line)) return 'OK';
  return 'INFO';
}

export default function LogsPage() {
  const { t } = useTranslation();
  const [logs, setLogs] = useState<string[]>([]);
  const [filter, setFilter] = useState('ALL');
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const raw = await invoke<string[]>('get_operation_logs', { lines: 500 });
      setLogs(raw);
    } catch { setLogs([]); }
    setLoading(false);
  }, []);

  useEffect(() => { refresh(); }, []);

  const filtered = logs.filter(l => {
    if (filter === 'ALL') return true;
    return parseLevelLine(l) === filter;
  });

  return (
    <div className="flex flex-col h-full min-h-0">
      <TopBar title={t('nav.logs')} subtitle={t('logs.subtitle')} />
      <div className="flex-1 overflow-y-auto px-5 py-4 space-y-4">
        <div className="rounded-xl border border-border bg-card overflow-hidden">
          <div className="flex items-center gap-3 px-5 py-3">
            <ScrollTextIcon className="w-4 h-4 text-primary" />
            <span className="text-sm font-semibold text-foreground">{t('logs.operationLog')}</span>
            <div className="flex items-center gap-1 ml-auto">
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
            {['ALL', 'OK', 'INFO', 'WARN', 'ERR'].map(level => {
              const color = LEVEL_COLORS[level] || '#888';
              const Icon = LEVEL_ICONS[level] || InfoIcon;
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
            <span className="ml-auto text-[10px] text-muted-foreground font-mono">{filtered.length} entries</span>
          </div>

          {/* Log lines */}
          <div className="divide-y divide-border/30 bg-background font-mono text-xs max-h-[calc(100vh-260px)] overflow-y-auto">
            {filtered.map((line, i) => {
              const level = parseLevelLine(line);
              const color = LEVEL_COLORS[level] || LEVEL_COLORS.INFO;
              const Icon = LEVEL_ICONS[level] || InfoIcon;
              return (
                <div key={i} className="flex items-start gap-2 px-4 py-2 hover:bg-muted/10">
                  <Icon className="w-3.5 h-3.5 mt-0.5 shrink-0" style={{ color }} />
                  <span style={{ color }} className="whitespace-pre-wrap break-all">{line}</span>
                </div>
              );
            })}
            {logs.length === 0 && !loading && (
              <div className="px-4 py-12 text-center text-muted-foreground text-xs">
{t('logs.empty')}
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
