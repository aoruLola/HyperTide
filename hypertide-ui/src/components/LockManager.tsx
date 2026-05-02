import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Lock, Unlock, AlertCircle, Loader2, ShieldCheck, HardDrive } from 'lucide-react';
import {
  Button,
  Input,
  Card,
  CardBody,
  Chip,
  Tooltip,
} from '@heroui/react';
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
    <div className="max-w-4xl mx-auto space-y-8 animate-in fade-in slide-in-from-bottom-6 duration-700">
      {/* 头部宣传位 */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-black text-gray-900 tracking-tight">Security Vault</h1>
          <p className="text-sm font-medium text-gray-400 mt-1">Manage asset locks and prevent data conflicts in real-time.</p>
        </div>
        <div className="w-12 h-12 rounded-2xl bg-orange-50 flex items-center justify-center border border-orange-100 shadow-sm">
          <ShieldCheck className="w-6 h-6 text-orange-500" />
        </div>
      </div>

      {/* 锁定操作卡片 */}
      <Card className="border-none shadow-xl shadow-gray-200/50 bg-white/80 backdrop-blur-md">
        <CardBody className="p-8">
          <div className="flex items-center gap-3 mb-6">
            <div className="w-8 h-8 rounded-lg bg-gray-900 flex items-center justify-center">
              <Lock className="w-4 h-4 text-white" />
            </div>
            <h2 className="text-lg font-bold text-gray-900">Acquire New Lock</h2>
          </div>

          <div className="flex flex-col gap-4">
            <div className="flex gap-3">
              <Input
                value={filePath}
                onChange={(e) => setFilePath(e.target.value)}
                placeholder="Enter file path (e.g. assets/models/hero.fbx)"
                className="flex-1"
                size="lg"
                classNames={{
                  inputWrapper: "bg-gray-50 border-gray-100 group-data-[focus=true]:bg-white group-data-[focus=true]:ring-2 group-data-[focus=true]:ring-orange-500/20",
                }}
              />
              <Button
                onClick={() => lockMutation.mutate(filePath)}
                isLoading={lockMutation.isPending}
                isDisabled={!filePath}
                color="primary"
                size="lg"
                className="px-8 font-black rounded-xl shadow-lg shadow-orange-200"
                startContent={!lockMutation.isPending && <Lock className="w-4 h-4" />}
              >
                LOCK
              </Button>
            </div>
            {lockMutation.isError && (
              <div className="px-4 py-3 rounded-xl bg-red-50 border border-red-100 flex items-center gap-3 text-red-600 text-sm font-bold animate-in zoom-in-95 duration-200">
                <AlertCircle className="w-4 h-4" />
                <span>{(lockMutation.error as any)?.response?.data?.error || 'Operation failed'}</span>
              </div>
            )}
          </div>
        </CardBody>
      </Card>

      {/* 活跃锁定列表 */}
      <div className="space-y-4">
        <div className="flex items-center justify-between px-2">
          <div className="flex items-center gap-2">
            <h3 className="text-sm font-black text-gray-400 uppercase tracking-widest">Active Locks</h3>
            <Chip size="sm" variant="flat" className="bg-gray-100 text-gray-500 font-bold">{locks?.length || 0}</Chip>
          </div>
        </div>

        {isLoading ? (
          <div className="flex flex-col items-center justify-center py-20 bg-gray-50/50 rounded-[40px] border border-dashed border-gray-200">
            <Loader2 className="w-10 h-10 animate-spin text-orange-400 mb-4" />
            <p className="text-sm font-bold text-gray-400">Syncing with server...</p>
          </div>
        ) : locks && locks.length > 0 ? (
          <div className="grid grid-cols-1 gap-3">
            {locks.map((lock) => (
              <Card
                key={lock.file_path}
                className="border-none shadow-md shadow-gray-100/50 hover:shadow-xl hover:shadow-gray-200/50 transition-all duration-300 group"
              >
                <CardBody className="p-5">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-4">
                      <div className="w-12 h-12 rounded-2xl bg-gray-50 flex items-center justify-center group-hover:bg-orange-50 transition-colors">
                        <HardDrive className="w-5 h-5 text-gray-400 group-hover:text-orange-500 transition-colors" />
                      </div>
                      <div>
                        <div className="text-base font-bold text-gray-900 leading-none mb-1.5">{lock.file_path}</div>
                        <div className="flex items-center gap-2">
                          <Chip size="sm" variant="flat" className="bg-orange-50 text-orange-600 font-bold text-[10px]">@{lock.owner_id}</Chip>
                          <span className="text-[10px] font-bold text-gray-300 uppercase leading-none">{formatDate(lock.locked_at)}</span>
                        </div>
                      </div>
                    </div>

                    <div className="flex gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
                      {lock.owner_id === userId && (
                        <Tooltip content="Unlock your asset">
                          <Button
                            onClick={() => unlockMutation.mutate(lock.file_path)}
                            isLoading={unlockMutation.isPending}
                            color="success"
                            variant="flat"
                            size="sm"
                            className="font-bold rounded-lg text-green-700 bg-green-50"
                            isIconOnly
                          >
                            <Unlock className="w-4 h-4" />
                          </Button>
                        </Tooltip>
                      )}
                      <Tooltip content="Force unlock (Warning)">
                        <Button
                          onClick={() => forceUnlockMutation.mutate(lock.file_path)}
                          isLoading={forceUnlockMutation.isPending}
                          color="danger"
                          variant="flat"
                          size="sm"
                          className="font-bold rounded-lg text-red-700 bg-red-50"
                          isIconOnly
                        >
                          <AlertCircle className="w-4 h-4" />
                        </Button>
                      </Tooltip>
                    </div>
                  </div>
                </CardBody>
              </Card>
            ))}
          </div>
        ) : (
          <div className="flex flex-col items-center justify-center py-20 bg-gray-50/50 rounded-[40px] border border-dashed border-gray-200 opacity-60">
            <Unlock className="w-12 h-12 text-gray-300 mb-4" />
            <p className="text-lg font-black text-gray-400">No Locks Found</p>
            <p className="text-xs font-medium text-gray-300">The system is currently free of any restrictions.</p>
          </div>
        )}
      </div>
    </div>
  );
}
