import { useQuery } from '@tanstack/react-query';
import { apiClient } from '../lib/api';
import { Server, HardDrive, Lock } from 'lucide-react';

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
    <footer data-testid="status-strip" className="status-strip shell-panel">
      <div className="flex items-center gap-3 md:gap-5">
        <span className="status-item">
          <Server className="h-3.5 w-3.5" />
          <span>localhost:3000</span>
        </span>
        <span className="status-item">
          <Lock className="h-3.5 w-3.5" />
          <span>{locks?.length ?? 0} locks</span>
        </span>
      </div>

      <span className="status-item">
        <HardDrive className="h-3.5 w-3.5" />
        <span>local storage</span>
      </span>
    </footer>
  );
}
