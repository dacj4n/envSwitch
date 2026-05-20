import { useRef } from 'react';
import { BrowserRouter, Routes, Route, useLocation } from 'react-router-dom';
import { Toaster } from 'sonner';
import './i18n';
import Sidebar from './components/Sidebar';
import InstallPage from './pages/Install';
import VersionsPage from './pages/Versions';
import ServicesPage from './pages/Services';
import StatusPage from './pages/Status';
import LogsPage from './pages/Logs';
import DoctorPage from './pages/Doctor';
import SettingsPage from './pages/Settings';

const PAGES = [
  { path: '/', Component: VersionsPage },
  { path: '/services', Component: ServicesPage },
  { path: '/status', Component: StatusPage },
  { path: '/logs', Component: LogsPage },
  { path: '/doctor', Component: DoctorPage },
  { path: '/settings', Component: SettingsPage },
];

const KeepAlive = ({ path, component: C }: { path: string; component: React.ComponentType }) => {
  const location = useLocation();
  const mounted = useRef(false);
  if (location.pathname === path) mounted.current = true;
  if (!mounted.current) return null;
  return (
    <div className="absolute inset-0" style={{ display: location.pathname === path ? undefined : 'none' }}>
      <C />
    </div>
  );
};

const Layout = () => (
  <div className="flex h-full w-full overflow-hidden bg-background">
    <Sidebar />
    <main className="flex-1 overflow-hidden bg-background relative">
      {PAGES.map(({ path, Component }) => (
        <KeepAlive key={path} path={path} component={Component} />
      ))}
    </main>
  </div>
);

const App = () => (
  <BrowserRouter>
    <Toaster position="bottom-right" theme="dark" />
    <Routes>
      <Route path="/install/:jobId" element={<InstallPage />} />
      <Route path="*" element={<Layout />} />
    </Routes>
  </BrowserRouter>
);

export default App;
