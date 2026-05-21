import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import TopBar from '../components/TopBar';
import { GlobeIcon, NetworkIcon, TerminalIcon, CopyIcon, CheckCircleIcon, XCircleIcon, Loader2Icon, ZapIcon } from 'lucide-react';

export default function SettingsPage() {
  const { t, i18n } = useTranslation();
  const [proxy, setProxy] = useState('');
  const [language, setLanguage] = useState(i18n.language?.startsWith('zh') ? 'zh' : 'en');
  const [initStatus, setInitStatus] = useState<{cli_available: boolean; shell_initialized: boolean; init_shell: string} | null>(null);
  const [initBusy, setInitBusy] = useState(false);

  useEffect(() => {
    invoke<string | null>('get_proxy').then(p => setProxy(p || '')).catch(() => {});
    refreshInitStatus();
  }, []);

  const refreshInitStatus = () => {
    invoke<{cli_available: boolean; shell_initialized: boolean; init_shell: string}>('check_init_status')
      .then(setInitStatus).catch(() => {});
  };

  const handleLanguageChange = (lang: string) => {
    setLanguage(lang);
    i18n.changeLanguage(lang);
    toast.success(lang === 'zh' ? t('settings.switchedZh') : t('settings.switchedEn'));
  };

  const doInit = async () => {
    setInitBusy(true);
    try {
      const msg = await invoke<string>('init_shell');
      toast.success(msg);
      refreshInitStatus();
    } catch (e) { toast.error(`${e}`); }
    setInitBusy(false);
  };

  const doUninit = async () => {
    setInitBusy(true);
    try {
      const msg = await invoke<string>('uninit_shell');
      toast.success(msg);
      refreshInitStatus();
    } catch (e) { toast.error(`${e}`); }
    setInitBusy(false);
  };

  return (
    <div className="flex flex-col h-full min-h-0">
      <TopBar title={t('settings.title')} subtitle={t('settings.subtitle')} />
      <div className="flex-1 overflow-y-auto px-5 py-4 space-y-5">
        {/* Shell Integration */}
        <div className="rounded-xl border border-border bg-card overflow-hidden">
          <div className="flex items-center gap-2 px-5 py-4 border-b border-border">
            <ZapIcon className="w-4 h-4 text-primary" />
            <span className="font-semibold text-sm text-foreground">{t('settings.shellIntegration')}</span>
          </div>
          <div className="px-5 py-4">
            <p className="text-xs text-muted-foreground mb-3">{t('settings.shellDesc')}</p>
            <div className="flex items-center gap-3 mb-3">
              <span className="text-xs text-muted-foreground">{t('settings.status')}:</span>
              {initStatus ? (
                <span className={`flex items-center gap-1.5 text-xs font-medium ${
                  initStatus.shell_initialized ? 'text-success' : 'text-destructive'
                }`}>
                  {initStatus.shell_initialized
                    ? <><CheckCircleIcon className="w-3.5 h-3.5" /> {initStatus.init_shell}</>
                    : <><XCircleIcon className="w-3.5 h-3.5" /> {t('settings.notInit')}</>
                  }
                </span>
              ) : (
                <Loader2Icon className="w-3 h-3 animate-spin text-muted-foreground" />
              )}
            </div>
            <div className="flex gap-2">
              <button onClick={doInit} disabled={initBusy}
                className="flex items-center gap-1.5 px-4 py-2 rounded-md bg-success/10 text-success border border-success/30 text-sm font-medium hover:bg-success/20 disabled:opacity-40"
              >{initBusy ? <Loader2Icon className="w-3.5 h-3.5 animate-spin" /> : <ZapIcon className="w-3.5 h-3.5" />} {t('settings.initBtn')}</button>
              <button onClick={doUninit} disabled={initBusy}
                className="flex items-center gap-1.5 px-4 py-2 rounded-md bg-destructive/10 text-destructive border border-destructive/30 text-sm font-medium hover:bg-destructive/20 disabled:opacity-40"
              >{t('settings.uninitBtn')}</button>
            </div>
          </div>
        </div>

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
              <button onClick={() => { invoke('set_proxy', { proxy }); toast.success(t('settings.proxySaved')); }}
                className="px-4 py-2 rounded-md bg-primary/15 text-primary border border-primary/30 text-sm font-medium hover:bg-primary/20">{t('common.save')}</button>
            </div>
          </div>
        </div>

        {/* CLI Examples */}
        <div className="rounded-xl border border-border bg-card overflow-hidden">
          <div className="flex items-center gap-2 px-5 py-4 border-b border-border">
            <TerminalIcon className="w-4 h-4 text-primary" />
            <span className="font-semibold text-sm text-foreground">{t('settings.cliExamples')}</span>
            <span className="ml-auto text-[10px] text-muted-foreground font-mono">{10} {t('common.commands')}</span>
          </div>
          <div className="divide-y divide-border/50 bg-background font-mono">
            {[
                { cmd: 'envswitch search jdk', desc: t('cli.searchJdk') },
                { cmd: 'envswitch install jdk 21.0.11', desc: t('cli.installJdk') },
                { cmd: 'envswitch cover jdk 21.0.11', desc: t('cli.coverJdk') },
                { cmd: 'envswitch cover go 1.25.10', desc: t('cli.coverGo') },
                { cmd: 'envswitch status', desc: t('cli.status') },
                { cmd: 'envswitch uncover --all', desc: t('cli.uncoverAll') },
                { cmd: 'envswitch list', desc: t('cli.list') },
                { cmd: 'envswitch doctor', desc: t('cli.doctor') },
                { cmd: 'envswitch start mysql 8.0', desc: t('cli.startMysql') },
                { cmd: 'envswitch cd-hook on', desc: t('cli.cdHook') },
              ].map((ex, i) => (
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
