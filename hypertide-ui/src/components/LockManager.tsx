import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Lock, Unlock, AlertCircle, Loader2 } from 'lucide-react';
import { apiClient } from '../lib/api';
import { useAppStore } from '../store/useAppStore';
import { formatDate } from '../lib/utils';

export function LockManager() {
  const [filePath, setFilePath] = useState('');
  const { userId } = useAppStore();
  const queryClient = useQueryClient();

  const { data: locks, isLoading } = useQuery({
    queryKey: ['locks'],
    queryFn: async () => {
      const res = await apiClient.locks.list();
      return res.data;
    },
    refetchInterval: 3000,
  });

  const lockMutation = useMutation({
    mutationFn: (path: string) => apiClient.locks.lock(path, userId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['locks'] });
      setFilePath('');
    },
  });

  const unlockMutation = useMutation({
    mutationFn: (path: string) => apiClient.locks.unlock(path, userId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['locks'] });
    },
  });

  const forceUnlockMutation = useMutation({
    mutationFn: (path: string) => apiClient.locks.forceUnlock(path),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['locks'] });
    },
  });

  return (
    <div className="space-y-6">
      {/* Lock Form */}
      <div className="bg-black/30 backdrop-blur-sm rounded-lg border border-purple-500/20 p-6">
        <h2 className="text-xl font-semibold text-white mb-4">锁定文件</h2>
        <div className="flex gap-3">
          <input
            type="text"
            value={filePath}
            onChange={(e) => setFilePath(e.target.value)}
            placeholder="输入文件路径，例如: assets/models/character.fbx"
            className="flex-1 px-4 py-2 bg-black/50 border border-purple-500/30 rounded-lg text-white placeholder-gray-500 focus:outline-none focus:border-purple-500"
          />
          <button
            onClick={() => lockMutation.mutate(filePath)}
            disabled={!filePath || lockMutation.isPending}
            className="px-6 py-2 bg-purple-600 hover:bg-purple-700 disabled:bg-gray-600 disabled:cursor-not-allowed text-white rounded-lg font-medium transition-colors flex items-center gap-2"
          >
            {lockMutation.isPending ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <Lock className="w-4 h-4" />
            )}
            锁定
          </button>
        </div>
        {lockMutation.isError && (
          <div className="mt-3 flex items-center gap-2 text-red-400 text-sm">
            <AlertCircle className="w-4 h-4" />
            <span>{(lockMutation.error as any)?.response?.data?.error || '锁定失败'}</span>
          </div>
        )}
      </div>

      {/* Locks List */}
      <div className="bg-black/30 backdrop-blur-sm rounded-lg border border-purple-500/20 p-6">
        <h2 className="text-xl font-semibold text-white mb-4">当前锁定</h2>
        {isLoading ? (
          <div className="flex items-center justify-center py-8">
            <Loader2 className="w-6 h-6 animate-spin text-purple-400" />
          </div>
        ) : locks && locks.length > 0 ? (
          <div className="space-y-3">
            {locks.map((lock) => (
              <div
                key={lock.file_path}
                className="bg-black/40 rounded-lg p-4 border border-purple-500/10 hover:border-purple-500/30 transition-colors"
              >
                <div className="flex items-start justify-between">
                  <div className="flex-1">
                    <div className="text-white font-medium mb-1">{lock.file_path}</div>
                    <div className="text-sm text-gray-400">
                      锁定者: {lock.owner_id} · {formatDate(lock.locked_at)}
                    </div>
                  </div>
                  <div className="flex gap-2">
                    {lock.owner_id === userId && (
                      <button
                        onClick={() => unlockMutation.mutate(lock.file_path)}
                        disabled={unlockMutation.isPending}
                        className="px-3 py-1.5 bg-green-600 hover:bg-green-700 disabled:bg-gray-600 text-white text-sm rounded-md transition-colors flex items-center gap-1.5"
                      >
                        <Unlock className="w-3.5 h-3.5" />
                        解锁
                      </button>
                    )}
                    <button
                      onClick={() => forceUnlockMutation.mutate(lock.file_path)}
                      disabled={forceUnlockMutation.isPending}
                      className="px-3 py-1.5 bg-red-600 hover:bg-red-700 disabled:bg-gray-600 text-white text-sm rounded-md transition-colors flex items-center gap-1.5"
                    >
                      <AlertCircle className="w-3.5 h-3.5" />
                      强制解锁
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-center py-8 text-gray-400">暂无锁定的文件</div>
        )}
      </div>
    </div>
  );
}
