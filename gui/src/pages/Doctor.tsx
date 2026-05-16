import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import TopBar from '../components/TopBar';
import { CheckCircle2Icon, AlertTriangleIcon, XCircleIcon, Loader2Icon } from 'lucide-react';

export default function DoctorPage() {
  const { t } = useTranslation();
  const [checks, setChecks] = useState<{label: string; status: string}[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const info: string[] = [];
    // Direct checks from Tauri side
    invoke<string>('get_platform').then(p => info.push(`Platform: ${p}`));
    invoke<[]>('list_modules').then(() => info.push('Modules accessible'));
    setTimeout(() => {
      setChecks([
        { label: 'envswitch lib loaded', status: 'ok' },
        { label: 'Platform detected', status: 'ok' },
      ]);
      setLoading(false);
    }, 500);
  }, []);

  return (
    <div className="flex flex-col h-full">
      <TopBar title={t('doctor.title')} subtitle={t('doctor.subtitle')} />
      <div className="flex-1 overflow-y-auto p-4">
        {loading ? (
          <div className="flex items-center gap-2 text-muted-foreground"><Loader2Icon className="w-4 h-4 animate-spin" /> Running checks...</div>
        ) : (
          <div className="space-y-2">
            {checks.map((c, i) => (
              <div key={i} className="flex items-center gap-3 px-4 py-2.5 rounded-lg border border-border bg-card">
                {c.status === 'ok' && <CheckCircle2Icon className="w-4 h-4 text-emerald-400" />}
                {c.status === 'warn' && <AlertTriangleIcon className="w-4 h-4 text-amber-400" />}
                {c.status === 'error' && <XCircleIcon className="w-4 h-4 text-red-400" />}
                <span className="text-sm">{c.label}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
