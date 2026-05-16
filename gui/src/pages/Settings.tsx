import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import TopBar from '../components/TopBar';
import { GlobeIcon, NetworkIcon } from 'lucide-react';

export default function SettingsPage() {
  const { t, i18n } = useTranslation();
  const [proxy, setProxy] = useState('');
  const [language, setLanguage] = useState(i18n.language?.startsWith('zh') ? 'zh' : 'en');

  const handleLanguageChange = (lang: string) => {
    setLanguage(lang);
    i18n.changeLanguage(lang);
    toast.success(lang === 'zh' ? '已切换为中文' : 'Switched to English');
  };

  const handleProxySave = () => {
    localStorage.setItem('envswitch_proxy', proxy);
    toast.success(t('toast.covered'));
  };

  return (
    <div className="flex flex-col h-full">
      <TopBar title={t('settings.title')} subtitle={t('settings.subtitle')} />
      <div className="flex-1 overflow-y-auto px-6 py-5 space-y-5">
        {/* Language */}
        <div className="rounded-xl border border-border bg-card overflow-hidden">
          <div className="flex items-center gap-2 px-5 py-4 border-b border-border">
            <GlobeIcon className="w-4 h-4 text-primary" />
            <span className="font-semibold text-sm text-foreground">{t('settings.language')}</span>
          </div>
          <div className="px-5 py-4">
            <p className="text-xs text-muted-foreground mb-3">{t('settings.languageDesc')}</p>
            <div className="flex gap-2">
              {[
                { value: 'en', label: 'English' },
                { value: 'zh', label: '中文' },
              ].map(({ value, label }) => (
                <button
                  key={value}
                  onClick={() => handleLanguageChange(value)}
                  className={`px-4 py-2 rounded-md text-sm font-medium transition-colors ${
                    language === value
                      ? 'bg-primary text-primary-foreground'
                      : 'bg-secondary hover:bg-secondary/80 text-secondary-foreground'
                  }`}
                >
                  {label}
                </button>
              ))}
            </div>
          </div>
        </div>

        {/* Proxy */}
        <div className="rounded-xl border border-border bg-card overflow-hidden">
          <div className="flex items-center gap-2 px-5 py-4 border-b border-border">
            <NetworkIcon className="w-4 h-4 text-primary" />
            <span className="font-semibold text-sm text-foreground">{t('settings.proxy')}</span>
          </div>
          <div className="px-5 py-4">
            <p className="text-xs text-muted-foreground mb-3">{t('settings.proxyDesc')}</p>
            <div className="flex gap-2">
              <input
                type="text" value={proxy}
                onChange={e => setProxy(e.target.value)}
                placeholder="http://127.0.0.1:7890"
                className="flex-1 px-3 py-2 rounded-md border border-border bg-background text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:border-primary"
              />
              <button onClick={handleProxySave}
                className="px-4 py-2 rounded-md bg-primary text-primary-foreground text-sm font-medium hover:bg-primary/90"
              >
                {t('common.save')}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
