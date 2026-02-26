import { useQuery } from '@tanstack/react-query';
import { apiClient } from '../lib/api';
import { Server, HardDrive } from 'lucide-react';

export function StatusBar() {
  const { data: locks } = useQuery({
    queryKey: ['locks'],
    queryFn: async () => {
      const res = await apiClient.locks.list();
      return res.data;
    },
    refetchInterval: 5000,
  });

  return (
    <div className="h-6 bg-gray-950 border-t border-gray-800 flex items-center justify-between px-4 text-xs text-gray-400">
      {/* Left */}
      <div className="flex items-center gap-4">
        <div className="flex items-center gap-1.5">
          <Server className="w-3 h-3" />
          <span>localhost:3000</span>
        </div>
        <div className="flex items-center gap-1.5">
          <Lock className="w-3 h-3" />
          <span>{locks?.length || 0} 个锁定</span>
        </div>
      </div>

      {/* Right */}
      <div className="flex items-center gap-1.5">
        <HardDrive className="w-3 h-3" />
        <span>本地存储</span>
      </div>
    </div>
  );
}
