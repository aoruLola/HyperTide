import { Settings, Wifi, WifiOff, PanelLeft, Radar, FlaskConical } from 'lucide-react';
import { useAppStore } from '../store/useAppStore';
import { useQuery } from '@tanstack/react-query';
import { useLocation } from 'react-router-dom';
import { apiClient } from '../lib/api';
import { DRY_RUN } from '../lib/config';
import { Button, Avatar } from '@heroui/react';

interface TopbarProps {
  mobileNavOpen: boolean;
  onToggleNavigation: () => void;
}

function resolveSection(pathname: string) {
  if (pathname.startsWith('/locks')) return { title: 'Lock Command', hint: 'synchronization guards' };
  if (pathname.startsWith('/upload')) return { title: 'Upload Bay', hint: 'ingest and checksum' };
  if (pathname.startsWith('/download')) return { title: 'Retrieval Desk', hint: 'hash-based delivery' };
  if (pathname.startsWith('/search')) return { title: 'Asset Search', hint: 'path and digest index' };
  if (pathname.startsWith('/history')) return { title: 'Activity Feed', hint: 'event timeline' };
  if (pathname.startsWith('/keys')) return { title: 'Key Authority', hint: 'credential governance' };
  return { title: 'Workspace Hub', hint: 'project asset cockpit' };
}

export function Topbar({ mobileNavOpen, onToggleNavigation }: TopbarProps) {
  const { userId } = useAppStore();
  const location = useLocation();

  const { data: health, refetch, isRefetching } = useQuery({
    queryKey: ['health'],
    queryFn: async () => {
      const res = await apiClient.health();
      return res.data;
    },
    refetchInterval: 10000,
    enabled: !DRY_RUN,
  });

  const section = resolveSection(location.pathname);

  return (
    <header data-testid="command-bar" className="command-bar shell-panel">
      <div className="command-primary">
        <Button
          isIconOnly
          size="sm"
          variant="flat"
          aria-label="Toggle navigation"
          aria-expanded={mobileNavOpen}
          onClick={onToggleNavigation}
          className="command-menu-btn lg:hidden"
        >
          <PanelLeft className="h-4 w-4" />
        </Button>

        <div className="command-brand">
          <span className="command-brand-icon">
            <Radar className="h-4 w-4" />
          </span>
          <div className="min-w-0">
            <p className="command-kicker">Hypertide Control</p>
            <div className="command-title-row">
              <h1 className="command-title">{section.title}</h1>
              <span className="command-hint">{section.hint}</span>
            </div>
          </div>
        </div>
      </div>

      <div className="command-actions">
        {DRY_RUN && (
          <span className="command-pill command-pill-warning">
            <FlaskConical className="h-3.5 w-3.5" />
            Demo Mode
          </span>
        )}

        {!DRY_RUN && (
          <Button
            size="sm"
            variant="flat"
            isIconOnly
            onClick={() => refetch()}
            isLoading={isRefetching}
            className="command-icon-btn"
            title="Refresh connection"
          >
            {health ? (
              <Wifi className="h-4 w-4 text-emerald-300" />
            ) : (
              <WifiOff className="h-4 w-4 text-rose-300" />
            )}
          </Button>
        )}

        <div className="command-user">
          <Avatar
            size="sm"
            name={userId}
            className="h-7 w-7 bg-gradient-to-br from-[#f09a3e] to-[#e55e28] text-[10px] font-bold text-white"
          />
          <span className="command-user-id">{userId}</span>
        </div>

        <Button
          size="sm"
          variant="flat"
          isIconOnly
          className="command-icon-btn"
          title="Settings"
        >
          <Settings className="h-4 w-4" />
        </Button>
      </div>
    </header>
  );
}
