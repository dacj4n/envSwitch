import { useTranslation } from 'react-i18next';
import TopBar from '../components/TopBar';
import { ScrollTextIcon } from 'lucide-react';

export default function LogsPage() {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col h-full">
      <TopBar title={t('nav.logs')} subtitle="Service logs (coming soon)" />
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center">
          <ScrollTextIcon className="w-12 h-12 text-muted-foreground/30 mx-auto mb-3" />
          <p className="text-sm text-muted-foreground">Log viewer coming in next release</p>
        </div>
      </div>
    </div>
  );
}
