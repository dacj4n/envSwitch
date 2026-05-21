import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import TopBar from '../components/TopBar';
import { usePageActive } from '../lib/utils';
import { CheckCircle2Icon, AlertTriangleIcon, XCircleIcon, Loader2Icon, HeartPulseIcon } from 'lucide-react';

interface CheckItem { label: string; status: 'ok' | 'warn' | 'error'; detail?: string; }

export default function DoctorPage() {
  const { t } = useTranslation();
  const [checks, setChecks] = useState<CheckItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [platform, setPlatform] = useState('');

  const runChecks = useCallback(() => {
    setLoading(true);
    Promise.all([
      invoke<string>('get_platform').catch(() => 'unknown'),
      invoke<any[]>('list_modules').catch(() => []),
      invoke<{cli_available: boolean; cli_path: string; shell_initialized: boolean; init_shell: string; home_dir_exists: boolean; shims_in_path: boolean}>('check_init_status').catch(() => null),
    ]).then(([plat, mods, initStatus]) => {
      setPlatform(plat);
      const items: CheckItem[] = [
        { label: t('doctor.platformDetected'), status: 'ok', detail: plat },
        { label: t('doctor.modulesLoaded'), status: 'ok', detail: `${(mods as any[]).length} ${t('doctor.countModules')}` },
        { label: t('doctor.cliAvailable'), status: initStatus?.cli_available ? 'ok' : 'warn', detail: initStatus?.cli_available ? initStatus.cli_path : t('doctor.cliOptional') },
        { label: t('doctor.shellInit'), status: initStatus?.shell_initialized ? 'ok' : 'error', detail: initStatus?.shell_initialized ? initStatus.init_shell : t('doctor.shellNotInit') },
        { label: t('doctor.shimsDir'), status: 'ok', detail: '~/.envswitch/shims/' },
        { label: t('doctor.brewAvailable'), status: mods ? 'ok' : 'warn', detail: mods ? t('doctor.brewFound') : t('doctor.brewNotFound') },
      ];
      setChecks(items);
      setLoading(false);
    });
  }, [t]);

  useEffect(() => { runChecks(); }, []);
  usePageActive('/doctor', runChecks);

  const okCount = checks.filter(c => c.status === 'ok').length;
  const warnCount = checks.filter(c => c.status === 'warn').length;
  const errCount = checks.filter(c => c.status === 'error').length;

  return (
    <div className="flex flex-col h-full min-h-0">
      <TopBar title={t('doctor.title')} subtitle={t('doctor.subtitle')} />

      <div className="flex-1 overflow-y-auto px-5 py-4">
        {loading ? (
          <div className="flex items-center gap-2 text-sm text-muted-foreground"><Loader2Icon className="w-4 h-4 animate-spin" /> {t('doctor.runningChecks')}</div>
        ) : (
          <div className="space-y-4">
            {/* Summary */}
            <div className="rounded-xl border border-border bg-card p-4 flex items-center gap-4">
              <HeartPulseIcon className="w-8 h-8 text-primary" />
              <div>
                <div className="text-sm font-semibold text-foreground">
                  {errCount > 0 ? `${errCount} ${t('doctor.issuesFound')}` : warnCount > 0 ? `${warnCount} ${t('doctor.warnings')}` : t('doctor.allOk')}
                </div>
                <div className="text-xs text-muted-foreground">{platform} · {okCount} {t('doctor.checksPassed')}</div>
              </div>
              <div className="ml-auto flex items-center gap-3">
                <div className="flex items-center gap-1 text-xs"><CheckCircle2Icon className="w-3.5 h-3.5 text-success" />{okCount}</div>
                <div className="flex items-center gap-1 text-xs"><AlertTriangleIcon className="w-3.5 h-3.5 text-warning" />{warnCount}</div>
                <div className="flex items-center gap-1 text-xs"><XCircleIcon className="w-3.5 h-3.5 text-destructive" />{errCount}</div>
              </div>
            </div>

            {/* Checks list */}
            <div className="rounded-xl border border-border bg-card overflow-hidden">
              <div className="divide-y divide-border/50">
                {checks.map((c, i) => (
                  <div key={i} className="flex items-center gap-3 px-5 py-3 hover:bg-muted/10 transition-colors">
                    {c.status === 'ok' && <CheckCircle2Icon className="w-4 h-4 text-success shrink-0" />}
                    {c.status === 'warn' && <AlertTriangleIcon className="w-4 h-4 text-warning shrink-0" />}
                    {c.status === 'error' && <XCircleIcon className="w-4 h-4 text-destructive shrink-0" />}
                    <span className="text-sm font-medium text-foreground">{c.label}</span>
                    {c.detail && <span className="text-xs text-muted-foreground font-mono ml-auto">{c.detail}</span>}
                    <span className={`text-[10px] px-1.5 py-0.5 rounded-full border font-medium ${
                      c.status === 'ok' ? 'bg-success/10 text-success border-success/25' :
                      c.status === 'warn' ? 'bg-warning/10 text-warning border-warning/25' :
                      'bg-destructive/10 text-destructive border-destructive/25'
                    }`}>{c.status}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
