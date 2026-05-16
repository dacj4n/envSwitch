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
    <div className="flex items-center justify-between px-6 py-4 border-b border-border shrink-0">
      <div>
        <h1 className="text-lg font-semibold text-foreground">{title}</h1>
        {subtitle && <p className="text-xs text-muted-foreground mt-0.5">{subtitle}</p>}
      </div>
      {onRefresh && (
        <button onClick={onRefresh}
          className="flex items-center gap-1.5 px-3 py-1.5 text-xs rounded-md bg-secondary hover:bg-secondary/80 text-secondary-foreground transition-colors"
        >
          <RefreshCwIcon className="w-3.5 h-3.5" />
          {t('common.refresh')}
        </button>
      )}
    </div>
  );
}
