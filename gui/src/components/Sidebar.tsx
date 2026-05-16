import { useEffect, useState } from 'react';
import { Link, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { LayersIcon, DatabaseIcon, ActivityIcon, ScrollTextIcon, HeartPulseIcon, SettingsIcon, ZapIcon } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

export default function Sidebar() {
  const { t } = useTranslation();
  const location = useLocation();
  const [platform, setPlatform] = useState('');

  useEffect(() => {
    invoke<string>('get_platform').then(setPlatform).catch(() => {});
  }, []);

  const NAV_ITEMS = [
    { path: '/', label: t('nav.versions'), icon: LayersIcon },
    { path: '/services', label: t('nav.services'), icon: DatabaseIcon },
    { path: '/status', label: t('nav.status'), icon: ActivityIcon },
    { path: '/logs', label: t('nav.logs'), icon: ScrollTextIcon },
    { path: '/doctor', label: t('nav.doctor'), icon: HeartPulseIcon },
  ];

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
        {NAV_ITEMS.map(({ path, label, icon: Icon }) => {
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

      <div className="px-3 py-4 border-t border-sidebar-border">
        <Link to="/settings"
          className={`flex items-center gap-3 px-3 py-2.5 rounded-md text-sm transition-all ${
            location.pathname === '/settings' ? 'bg-sidebar-accent text-foreground font-medium' : 'text-sidebar-foreground hover:bg-sidebar-accent/60'
          }`}
        >
          <SettingsIcon className="w-4 h-4 text-muted-foreground" />
          {t('nav.settings')}
        </Link>
      </div>
    </aside>
  );
}
