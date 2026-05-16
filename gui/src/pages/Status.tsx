import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import TopBar from '../components/TopBar';

interface ActiveCover { module_name: string; version: string; scope: string; applied_at: string; }

export default function StatusPage() {
  const { t } = useTranslation();
  const [covers, setCovers] = useState<ActiveCover[]>([]);
  const refresh = () => { invoke<ActiveCover[]>('get_status').then(setCovers); };
  useEffect(() => { refresh(); }, []);

  return (
    <div className="flex flex-col h-full">
      <TopBar title={t('nav.status')} subtitle="Active version covers" onRefresh={refresh} />
      <div className="flex-1 overflow-y-auto p-4">
        {covers.length === 0 ? (
          <p className="text-muted-foreground text-sm">No active covers</p>
        ) : (
          <div className="rounded-xl border border-border overflow-hidden">
            <table className="w-full text-sm">
              <thead className="bg-muted/50">
                <tr>
                  <th className="px-4 py-3 text-left text-xs font-medium text-muted-foreground">#</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-muted-foreground">Module</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-muted-foreground">Version</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-muted-foreground">Scope</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-muted-foreground">Applied</th>
                </tr>
              </thead>
              <tbody>
                {covers.map((c, i) => (
                  <tr key={i} className="border-t border-border">
                    <td className="px-4 py-3 text-muted-foreground font-mono text-xs">{i + 1}</td>
                    <td className="px-4 py-3 font-medium">{c.module_name}</td>
                    <td className="px-4 py-3 font-mono text-xs">{c.version}</td>
                    <td className="px-4 py-3">
                      <span className={`text-xs px-2 py-0.5 rounded ${c.scope === 'Global' ? 'bg-amber-500/15 text-amber-400' : 'bg-sky-500/15 text-sky-400'}`}>{c.scope}</span>
                    </td>
                    <td className="px-4 py-3 text-xs text-muted-foreground">{c.applied_at?.slice(0, 19).replace('T', ' ')}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
