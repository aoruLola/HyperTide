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

  const filteredLocks = locks?.filter(
    (lock) =>
      lock.file_path.toLowerCase().includes(searchTerm.toLowerCase()) ||
      lock.owner_id.toLowerCase().includes(searchTerm.toLowerCase()),
  );

  return (
    <div className="page-shell page-flat" data-testid="locks-flat-layout">
      <div className="page-header">
        <h1 className="page-title">Lock Management</h1>
        <p className="page-subtitle">Manage file locks and prevent editing conflicts.</p>
      </div>

      <section className="flat-section" data-testid="locks-actions-section">
        <div className="flat-toolbar">
          <Input
            placeholder="Enter file path to lock"
            value={filePath}
            onChange={(e) => setFilePath(e.target.value)}
            size="sm"
            classNames={{
              inputWrapper: 'bg-white border-gray-200',
            }}
          />
          <Button
            color="primary"
            onClick={() => lockMutation.mutate(filePath)}
            isDisabled={!filePath || lockMutation.isPending}
            isLoading={lockMutation.isPending}
            startContent={<Plus className="h-4 w-4" />}
            className="shrink-0"
          >
            Lock
          </Button>
        </div>
        <div className="mt-3">
          <Input
            placeholder="Search by file path or owner"
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            size="sm"
            startContent={<Search className="h-4 w-4 text-gray-400" />}
            classNames={{
              inputWrapper: 'bg-white border-gray-200',
            }}
          />
        </div>
      </section>

      <section className="flat-section flat-grow" data-testid="locks-table-section">
        <Table
          aria-label="Locked file list"
          removeWrapper
          classNames={{
            th: 'bg-transparent text-gray-700 font-semibold border-b border-black/10',
            td: 'text-gray-900 border-b border-black/5',
          }}
        >
          <TableHeader>
            <TableColumn>File Path</TableColumn>
            <TableColumn>Owner</TableColumn>
            <TableColumn>Locked At</TableColumn>
            <TableColumn align="end">Actions</TableColumn>
          </TableHeader>
          <TableBody
            items={filteredLocks || []}
            isLoading={isLoading}
            loadingContent={<Spinner />}
            emptyContent={
              <div className="flat-empty">
                <Lock className="mx-auto mb-3 h-10 w-10 opacity-30" />
                <p>No locked files.</p>
              </div>
            }
          >
            {(lock) => (
              <TableRow key={lock.file_path}>
                <TableCell>
                  <div className="flex items-center gap-2">
                    <Lock className="h-4 w-4 text-orange-500" />
                    <code className="text-sm text-gray-700">{lock.file_path}</code>
                  </div>
                </TableCell>
                <TableCell>
                  <Chip size="sm" variant="flat" className="bg-gray-100 text-gray-700">
                    {lock.owner_id}
                  </Chip>
                </TableCell>
                <TableCell>
                  <span className="text-sm text-gray-500">{formatDate(lock.locked_at)}</span>
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
                        startContent={<Unlock className="h-3 w-3" />}
                      >
                        Unlock
                      </Button>
                    )}
                    <Button
                      size="sm"
                      color="danger"
                      variant="flat"
                      onClick={() => {
                        if (confirm(`Force unlock ${lock.file_path}?`)) {
                          forceUnlockMutation.mutate(lock.file_path);
                        }
                      }}
                      isDisabled={forceUnlockMutation.isPending}
                      startContent={<AlertCircle className="h-3 w-3" />}
                    >
                      Force
                    </Button>
                  </div>
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </section>
    </div>
  );
}
