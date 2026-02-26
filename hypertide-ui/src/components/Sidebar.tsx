import { NavLink } from 'react-router-dom';
import {
  FolderTree,
  Lock,
  Upload,
  Download,
  Key,
  History,
  Search,
  Gauge,
} from 'lucide-react';

interface SidebarProps {
  mobileNavOpen: boolean;
  onCloseNavigation: () => void;
}

const navItems = [
  { path: '/', icon: Gauge, label: 'Workspace' },
  { path: '/locks', icon: Lock, label: 'Locks' },
  { path: '/upload', icon: Upload, label: 'Upload' },
  { path: '/download', icon: Download, label: 'Download' },
  { path: '/search', icon: Search, label: 'Search' },
  { path: '/history', icon: History, label: 'History' },
  { path: '/keys', icon: Key, label: 'API Keys' },
];

export function Sidebar({ mobileNavOpen, onCloseNavigation }: SidebarProps) {
  return (
    <>
      <button
        type="button"
        aria-label="Close navigation"
        className={`nav-backdrop ${mobileNavOpen ? 'is-open' : ''}`}
        onClick={onCloseNavigation}
      />

      <aside
        data-testid="workspace-nav"
        className={`workspace-nav shell-panel ${mobileNavOpen ? 'is-open' : ''}`}
      >
        <div className="nav-brand">
          <div className="nav-brand-badge">
            <FolderTree className="h-5 w-5" />
          </div>
          <div>
            <p className="nav-brand-kicker">Ops Deck</p>
            <h2 className="nav-brand-title">Hypertide</h2>
          </div>
        </div>

        <nav className="nav-stack" aria-label="Primary navigation">
          {navItems.map((item, index) => (
            <NavLink
              key={item.path}
              to={item.path}
              onClick={onCloseNavigation}
              style={{ animationDelay: `${index * 48}ms` }}
              className={({ isActive }) => `nav-link ${isActive ? 'is-active' : ''}`}
            >
              {({ isActive }) => (
                <>
                  <item.icon className={`h-4 w-4 ${isActive ? 'text-[#ffbd72]' : 'text-slate-300'}`} />
                  <span className="nav-link-label">{item.label}</span>
                </>
              )}
            </NavLink>
          ))}
        </nav>

        <div className="nav-meta">
          <p className="nav-meta-label">Cluster</p>
          <p className="nav-meta-title">Local Control Plane</p>
          <p className="nav-meta-text">Build 26.0.1</p>
        </div>
      </aside>
    </>
  );
}
