import { RefreshCwIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface TopBarProps {
  title: string;
  subtitle?: string;
  onRefresh?: () => void;
}

export default function TopBar({ title, subtitle, onRefresh }: TopBarProps) {
  const { t } = useTranslation();

  return (
    <header className="flex items-center justify-between px-6 py-4 bg-card border-b border-border shrink-0">
      <div>
        <h1 className="text-foreground font-semibold text-base leading-tight">{title}</h1>
        {subtitle && <p className="text-muted-foreground text-xs mt-0.5">{subtitle}</p>}
      </div>
      <div className="flex items-center gap-3">
        {onRefresh && (
          <button onClick={onRefresh}
            className="flex items-center gap-1.5 px-3 py-2 bg-secondary text-secondary-foreground rounded-md text-sm hover:bg-accent hover:text-accent-foreground transition-all border border-border"
          >
            <RefreshCwIcon className="w-3.5 h-3.5" />
            <span className="text-xs">{t('common.refresh')}</span>
          </button>
        )}
      </div>
    </header>
  );
}
