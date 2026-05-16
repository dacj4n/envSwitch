import { BrowserRouter, Routes, Route } from 'react-router-dom';
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

const App = () => (
  <BrowserRouter>
    <Toaster position="bottom-right" theme="dark" />
    <div className="flex h-full w-full overflow-hidden bg-background">
      <Sidebar />
      <main className="flex-1 overflow-hidden flex flex-col bg-background">
        <Routes>
          <Route path="/" element={<VersionsPage />} />
          <Route path="/install/:jobId" element={<InstallPage />} />
          <Route path="/services" element={<ServicesPage />} />
          <Route path="/status" element={<StatusPage />} />
          <Route path="/logs" element={<LogsPage />} />
          <Route path="/doctor" element={<DoctorPage />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Routes>
      </main>
    </div>
  </BrowserRouter>
);

export default App;
