import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import TopBar from '../components/TopBar';
import { GlobeIcon, NetworkIcon, TerminalIcon, CopyIcon } from 'lucide-react';

const CLI_EXAMPLES = [
  { cmd: 'envswitch search jdk', desc: 'Search available JDK versions' },
  { cmd: 'envswitch install jdk 21.0.11', desc: 'Install JDK 21' },
  { cmd: 'envswitch cover jdk 21.0.11', desc: 'Activate JDK 21 (shim switch)' },
  { cmd: 'envswitch cover go 1.25.10', desc: 'Activate Go 1.25.10' },
  { cmd: 'envswitch status', desc: 'Show current cover stack' },
  { cmd: 'envswitch uncover --all', desc: 'Restore all system defaults' },
  { cmd: 'envswitch list', desc: 'List installed versions' },
  { cmd: 'envswitch doctor', desc: 'Diagnose setup issues' },
  { cmd: 'envswitch start mysql 8.0', desc: 'Start MySQL service' },
  { cmd: 'envswitch cd-hook on', desc: 'Enable auto-switch on cd' },
];

export default function SettingsPage() {
  const { t, i18n } = useTranslation();
  const [proxy, setProxy] = useState(localStorage.getItem('envswitch_proxy') || '');
  const [language, setLanguage] = useState(i18n.language?.startsWith('zh') ? 'zh' : 'en');

  const handleLanguageChange = (lang: string) => {
    setLanguage(lang);
    i18n.changeLanguage(lang);
    toast.success(lang === 'zh' ? '已切换为中文' : 'Switched to English');
  };

  return (
    <div className="flex flex-col h-full min-h-0">
      <TopBar title={t('settings.title')} subtitle={t('settings.subtitle')} />
      <div className="flex-1 overflow-y-auto px-5 py-4 space-y-5">
        {/* Language */}
        <div className="rounded-xl border border-border bg-card overflow-hidden">
          <div className="flex items-center gap-2 px-5 py-4 border-b border-border">
            <GlobeIcon className="w-4 h-4 text-primary" />
            <span className="font-semibold text-sm text-foreground">{t('settings.language')}</span>
          </div>
          <div className="px-5 py-4">
            <p className="text-xs text-muted-foreground mb-3">{t('settings.languageDesc')}</p>
            <div className="flex gap-2">
              {[{ value: 'en', label: 'English' }, { value: 'zh', label: '中文' }].map(({ value, label }) => (
                <button key={value} onClick={() => handleLanguageChange(value)}
                  className={`px-4 py-2 rounded-md text-sm font-medium transition-colors border ${
                    language === value
                      ? 'bg-primary/15 text-primary border-primary/30'
                      : 'bg-secondary text-secondary-foreground border-border hover:bg-accent'
                  }`}>
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
              <input type="text" value={proxy} onChange={e => setProxy(e.target.value)}
                placeholder="http://127.0.0.1:7890"
                className="flex-1 px-3 py-2 rounded-md border border-border bg-background text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:border-primary" />
              <button onClick={() => { localStorage.setItem('envswitch_proxy', proxy); toast.success('Saved'); }}
                className="px-4 py-2 rounded-md bg-primary/15 text-primary border border-primary/30 text-sm font-medium hover:bg-primary/20">Save</button>
            </div>
          </div>
        </div>

        {/* CLI Examples */}
        <div className="rounded-xl border border-border bg-card overflow-hidden">
          <div className="flex items-center gap-2 px-5 py-4 border-b border-border">
            <TerminalIcon className="w-4 h-4 text-primary" />
            <span className="font-semibold text-sm text-foreground">CLI Examples</span>
            <span className="ml-auto text-[10px] text-muted-foreground font-mono">{CLI_EXAMPLES.length} commands</span>
          </div>
          <div className="divide-y divide-border/50 bg-background font-mono">
            {CLI_EXAMPLES.map((ex, i) => (
              <div key={i} className="flex items-center gap-3 px-5 py-3 hover:bg-muted/10 transition-colors group">
                <span className="text-primary text-xs shrink-0">$</span>
                <span className="text-sm text-foreground whitespace-nowrap">{ex.cmd}</span>
                <span className="text-[11px] text-muted-foreground ml-auto text-right">{ex.desc}</span>
                <button
                  onClick={() => { navigator.clipboard.writeText(ex.cmd); toast.success('Copied'); }}
                  className="opacity-0 group-hover:opacity-100 transition-opacity p-1 rounded hover:bg-muted/30 text-muted-foreground hover:text-foreground shrink-0"
                >
                  <CopyIcon className="w-3 h-3" />
                </button>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
