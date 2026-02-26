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
  Tooltip,
} from '@heroui/react';
import { Key, Plus, Trash2, Copy, CheckCircle, Search } from 'lucide-react';
import { apiClient } from '../lib/api';
import { formatDate } from '../lib/utils';

export function KeysPage() {
  const [ownerId, setOwnerId] = useState('');
  const [searchTerm, setSearchTerm] = useState('');
  const [copiedKey, setCopiedKey] = useState<string | null>(null);
  const queryClient = useQueryClient();

  const { data: keys, isLoading } = useQuery({
    queryKey: ['keys'],
    queryFn: async () => {
      const res = await apiClient.auth.listKeys();
      return res.data;
    },
  });

  const generateMutation = useMutation({
    mutationFn: (owner: string) =>
      apiClient.auth.generate(owner, ['Lock', 'Upload', 'Download'], null),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['keys'] });
      setOwnerId('');
    },
  });

  const revokeMutation = useMutation({
    mutationFn: (key: string) => apiClient.auth.revoke(key),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['keys'] });
    },
  });

  const copyToClipboard = (key: string) => {
    navigator.clipboard.writeText(key);
    setCopiedKey(key);
    setTimeout(() => setCopiedKey(null), 2000);
  };

  const filteredKeys = keys?.filter(key =>
    key.owner_id.toLowerCase().includes(searchTerm.toLowerCase()) ||
    key.key.toLowerCase().includes(searchTerm.toLowerCase())
  );

  return (
    <div className="h-full flex flex-col bg-background p-6 gap-4">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold text-foreground mb-2">API 密钥管理</h1>
        <p className="text-default-500">生成和管理 API 访问密钥</p>
      </div>

      {/* Toolbar */}
      <div className="flex gap-3">
        <Input
          placeholder="输入所有者 ID 生成密钥..."
          value={ownerId}
          onChange={(e) => setOwnerId(e.target.value)}
          className="flex-1"
        />
        <Button
          color="primary"
          onClick={() => generateMutation.mutate(ownerId)}
          isDisabled={!ownerId || generateMutation.isPending}
          isLoading={generateMutation.isPending}
          startContent={<Plus className="w-4 h-4" />}
        >
          生成
        </Button>
      </div>

      {/* Search */}
      <Input
        placeholder="搜索所有者或密钥..."
        value={searchTerm}
        onChange={(e) => setSearchTerm(e.target.value)}
        startContent={<Search className="w-4 h-4 text-default-400" />}
      />

      {/* Table */}
      <div className="flex-1 overflow-auto">
        <Table
          aria-label="API 密钥列表"
          classNames={{
            wrapper: "h-full",
          }}
        >
          <TableHeader>
            <TableColumn>密钥</TableColumn>
            <TableColumn>所有者</TableColumn>
            <TableColumn>权限</TableColumn>
            <TableColumn>创建时间</TableColumn>
            <TableColumn>状态</TableColumn>
            <TableColumn align="end">操作</TableColumn>
          </TableHeader>
          <TableBody
            items={filteredKeys || []}
            isLoading={isLoading}
            loadingContent={<Spinner />}
            emptyContent="暂无 API 密钥"
          >
            {(apiKey) => (
              <TableRow key={apiKey.key} className={apiKey.revoked ? 'opacity-50' : ''}>
                <TableCell>
                  <div className="flex items-center gap-2">
                    <code className="text-xs">{apiKey.key.substring(0, 20)}...</code>
                    <Tooltip content={copiedKey === apiKey.key ? '已复制!' : '复制密钥'}>
                      <Button
                        isIconOnly
                        size="sm"
                        variant="light"
                        onClick={() => copyToClipboard(apiKey.key)}
                      >
                        {copiedKey === apiKey.key ? (
                          <CheckCircle className="w-4 h-4 text-success" />
                        ) : (
                          <Copy className="w-4 h-4" />
                        )}
                      </Button>
                    </Tooltip>
                  </div>
                </TableCell>
                <TableCell>
                  <Chip size="sm" variant="flat">
                    {apiKey.owner_id}
                  </Chip>
                </TableCell>
                <TableCell>
                  <div className="flex gap-1 flex-wrap">
                    {apiKey.permissions.map(perm => (
                      <Chip key={perm} size="sm" variant="dot">
                        {perm}
                      </Chip>
                    ))}
                  </div>
                </TableCell>
                <TableCell>
                  <span className="text-sm text-default-500">
                    {formatDate(apiKey.created_at)}
                  </span>
                </TableCell>
                <TableCell>
                  {apiKey.revoked ? (
                    <Chip size="sm" color="danger" variant="flat">已撤销</Chip>
                  ) : (
                    <Chip size="sm" color="success" variant="flat">有效</Chip>
                  )}
                </TableCell>
                <TableCell>
                  {!apiKey.revoked && (
                    <Button
                      size="sm"
                      color="danger"
                      variant="flat"
                      onClick={() => {
                        if (confirm('确定要撤销此密钥吗？')) {
                          revokeMutation.mutate(apiKey.key);
                        }
                      }}
                      isDisabled={revokeMutation.isPending}
                      startContent={<Trash2 className="w-3 h-3" />}
                    >
                      撤销
                    </Button>
                  )}
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>
    </div>
  );
}
