import { useEffect, useState } from 'react';
import { Outlet, useLocation } from 'react-router-dom';
import { Sidebar } from '../components/Sidebar';
import { Topbar } from '../components/Topbar';
import { StatusBar } from '../components/StatusBar';

export function MainLayout() {
  const [mobileNavOpen, setMobileNavOpen] = useState(false);
  const location = useLocation();

  useEffect(() => {
    setMobileNavOpen(false);
  }, [location.pathname]);

  return (
    <div data-testid="app-shell" className="app-shell">
      <div data-testid="app-aurora" className="app-aurora" aria-hidden="true" />

      <Topbar
        mobileNavOpen={mobileNavOpen}
        onToggleNavigation={() => setMobileNavOpen((prev) => !prev)}
      />

      <div data-testid="layout-grid" className="layout-grid">
        <Sidebar
          mobileNavOpen={mobileNavOpen}
          onCloseNavigation={() => setMobileNavOpen(false)}
        />

        <main data-testid="content-stage" className="content-stage">
          <div className="content-scroll">
            <Outlet />
          </div>
        </main>
      </div>

      <StatusBar />
    </div>
  );
}
