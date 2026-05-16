import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import TopBar from '../components/TopBar';
import { ScrollTextIcon, InfoIcon, CheckCircle2Icon, AlertTriangleIcon, XCircleIcon } from 'lucide-react';

const LEVEL_COLORS: Record<string, string> = {
  ALL: '#60a5fa', INFO: '#60a5fa', SUCCESS: '#22c55e', WARN: '#f59e0b', ERROR: '#ef4444',
};
const LEVEL_ICONS: Record<string, React.ComponentType<any>> = {
  ALL: InfoIcon, INFO: InfoIcon, SUCCESS: CheckCircle2Icon, WARN: AlertTriangleIcon, ERROR: XCircleIcon,
};

export default function LogsPage() {
  const { t } = useTranslation();
  const [service, setService] = useState('mysql');
  const [filter, setFilter] = useState('ALL');
  const [lines, setLines] = useState(50);

  return (
    <div className="flex flex-col h-full min-h-0">
      <TopBar title={t('nav.logs')} subtitle="Service logs with color-coded levels" />
      <div className="flex-1 overflow-y-auto px-5 py-4 space-y-4">
        {/* Controls */}
        <div className="rounded-xl border border-border bg-card overflow-hidden">
          <div className="flex items-center gap-3 px-5 py-3">
            <ScrollTextIcon className="w-4 h-4 text-primary" />
            <span className="text-sm font-semibold text-foreground">Log Viewer</span>
            <div className="flex items-center gap-1 ml-auto">
              <select value={service} onChange={e => setService(e.target.value)}
                className="px-2.5 py-1.5 rounded-md border border-border bg-background text-xs text-foreground">
                <option value="mysql">mysql</option>
                <option value="pgsql">pgsql</option>
              </select>
              <input type="number" value={lines} onChange={e => setLines(Number(e.target.value))}
                className="w-16 px-2 py-1.5 rounded-md border border-border bg-background text-xs text-foreground" />
              <button
                className="px-3 py-1.5 rounded-md text-xs bg-secondary hover:bg-accent border border-border text-secondary-foreground"
              >Refresh</button>
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
          <div className="divide-y divide-border/30 bg-background font-mono text-xs">
            {[
              { text: '[2026-05-16 14:32:01] [INFO] Server started on port 3306', level: 'INFO' },
              { text: '[2026-05-16 14:32:02] [SUCCESS] InnoDB initialization completed', level: 'SUCCESS' },
              { text: '[2026-05-16 14:32:03] [WARN] Using default configuration file', level: 'WARN' },
              { text: '[2026-05-16 14:35:00] [ERROR] Connection refused from 192.168.1.100', level: 'ERROR' },
              { text: '[2026-05-16 14:36:00] [INFO] Graceful shutdown initiated', level: 'INFO' },
            ]
            .filter(l => filter === 'ALL' || l.level === filter)
            .map((line, i) => {
              const color = LEVEL_COLORS[line.level];
              const Icon = LEVEL_ICONS[line.level];
              return (
                <div key={i} className="flex items-start gap-2 px-4 py-2 hover:bg-muted/10">
                  <Icon className="w-3.5 h-3.5 mt-0.5 shrink-0" style={{ color }} />
                  <span style={{ color }} className="whitespace-pre-wrap break-all">{line.text}</span>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}
