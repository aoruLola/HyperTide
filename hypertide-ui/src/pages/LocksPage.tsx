import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  Table,
  TableHeader,
  TableColumn,
  TableBody,
  TableRow,
  TableCell,
  Input,
  Button,
  Chip,
  Spinner,
} from '@heroui/react';
import { Lock, Unlock, AlertCircle, Plus, Search } from 'lucide-react';
import { apiClient } from '../lib/api';
import { useAppStore } from '../store/useAppStore';
import { formatDate } from '../lib/utils';

export function LocksPage() {
  const [filePath, setFilePath] = useState('');
  const [searchTerm, setSearchTerm] = useState('');
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

  const filteredLocks = locks?.filter(lock =>
    lock.file_path.toLowerCase().includes(searchTerm.toLowerCase()) ||
    lock.owner_id.toLowerCase().includes(searchTerm.toLowerCase())
  );

  return (
    <div className="h-full flex flex-col bg-background p-6 gap-4">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold text-foreground mb-2">文件锁定管理</h1>
        <p className="text-default-500">管理文件锁定状态，防止并发编辑冲突</p>
      </div>

      {/* Toolbar */}
      <div className="flex gap-3">
        <Input
          placeholder="输入文件路径进行锁定..."
          value={filePath}
          onChange={(e) => setFilePath(e.target.value)}
          className="flex-1"
        />
        <Button
          color="primary"
          onClick={() => lockMutation.mutate(filePath)}
          isDisabled={!filePath || lockMutation.isPending}
          isLoading={lockMutation.isPending}
          startContent={<Plus className="w-4 h-4" />}
        >
          锁定
        </Button>
      </div>

      {/* Search */}
      <Input
        placeholder="搜索文件路径或所有者..."
        value={searchTerm}
        onChange={(e) => setSearchTerm(e.target.value)}
        startContent={<Search className="w-4 h-4 text-default-400" />}
      />

      {/* Table */}
      <div className="flex-1 overflow-auto">
        <Table
          aria-label="锁定文件列表"
          classNames={{
            wrapper: "h-full",
          }}
        >
          <TableHeader>
            <TableColumn>文件路径</TableColumn>
            <TableColumn>锁定者</TableColumn>
            <TableColumn>锁定时间</TableColumn>
            <TableColumn align="end">操作</TableColumn>
          </TableHeader>
          <TableBody
            items={filteredLocks || []}
            isLoading={isLoading}
            loadingContent={<Spinner />}
            emptyContent="暂无锁定的文件"
          >
            {(lock) => (
              <TableRow key={lock.file_path}>
                <TableCell>
                  <div className="flex items-center gap-2">
                    <Lock className="w-4 h-4 text-warning" />
                    <code className="text-sm">{lock.file_path}</code>
                  </div>
                </TableCell>
                <TableCell>
                  <Chip size="sm" variant="flat">
                    {lock.owner_id}
                  </Chip>
                </TableCell>
                <TableCell>
                  <span className="text-sm text-default-500">
                    {formatDate(lock.locked_at)}
                  </span>
                </TableCell>
                <TableCell>
                  <div className="flex items-center justify-end gap-2">
                    {lock.owner_id === userId && (
                      <Button
                        size="sm"
                        color="success"
                        variant="flat"
                        onClick={() => unlockMutation.mutate(lock.file_path)}
                        isDisabled={unlockMutation.isPending}
                        startContent={<Unlock className="w-3 h-3" />}
                      >
                        解锁
                      </Button>
                    )}
                    <Button
                      size="sm"
                      color="danger"
                      variant="flat"
                      onClick={() => {
                        if (confirm(`确定要强制解锁 ${lock.file_path} 吗？`)) {
                          forceUnlockMutation.mutate(lock.file_path);
                        }
                      }}
                      isDisabled={forceUnlockMutation.isPending}
                      startContent={<AlertCircle className="w-3 h-3" />}
                    >
                      强制
                    </Button>
                  </div>
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>
    </div>
  );
}
