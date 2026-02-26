import { useState } from 'react';
import { Outlet } from 'react-router-dom';
import { Sidebar } from '../components/Sidebar';
import { Topbar } from '../components/Topbar';
import { StatusBar } from '../components/StatusBar';

export function MainLayout() {
  const [sidebarWidth, setSidebarWidth] = useState(240);

  return (
    <div className="h-screen flex flex-col bg-gray-900">
      {/* Top Bar */}
      <Topbar />

      {/* Main Content Area */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Sidebar */}
        <Sidebar width={sidebarWidth} onResize={setSidebarWidth} />

        {/* Main Content */}
        <main className="flex-1 overflow-hidden">
          <Outlet />
        </main>
      </div>

      {/* Bottom Status Bar */}
      <StatusBar />
    </div>
  );
}
