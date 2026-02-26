import { Settings, User, RefreshCw } from 'lucide-react';
import { useAppStore } from '../store/useAppStore';
import { useQuery } from '@tanstack/react-query';
import { apiClient } from '../lib/api';

export function Topbar() {
  const { userId } = useAppStore();

  const { data: health, refetch } = useQuery({
    queryKey: ['health'],
    queryFn: async () => {
      const res = await apiClient.health();
      return res.data;
    },
    refetchInterval: 10000,
  });

  return (
    <div className="h-12 bg-gray-950 border-b border-gray-800 flex items-center justify-between px-4">
      {/* Left: Logo */}
      <div className="flex items-center gap-3">
        <div className="text-xl font-bold bg-gradient-to-r from-red-500 via-purple-500 to-purple-600 bg-clip-text text-transparent">
          HYPERTIDE
        </div>
        <div className="text-xs text-gray-500 font-mono">v26.0.1</div>
      </div>

      {/* Right: User & Actions */}
      <div className="flex items-center gap-4">
        {/* Connection Status */}
        <button
          onClick={() => refetch()}
          className="flex items-center gap-2 text-xs text-gray-400 hover:text-gray-300 transition-colors"
          title="刷新连接"
        >
          <div className={`w-2 h-2 rounded-full ${health ? 'bg-green-500' : 'bg-red-500'}`} />
          <span>{health ? '已连接' : '未连接'}</span>
          <RefreshCw className="w-3 h-3" />
        </button>

        {/* User */}
        <div className="flex items-center gap-2 text-sm text-gray-300">
          <User className="w-4 h-4" />
          <span>{userId}</span>
        </div>

        {/* Settings */}
        <button
          className="p-1.5 hover:bg-gray-800 rounded transition-colors"
          title="设置"
        >
          <Settings className="w-4 h-4 text-gray-400" />
        </button>
      </div>
    </div>
  );
}
