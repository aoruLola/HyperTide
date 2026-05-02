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
      apiClient.auth.generate(owner, ['Lock', 'Upload', 'Download']),
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

  const filteredKeys = Array.isArray(keys)
    ? keys.filter(
        (key) =>
          key.owner_id.toLowerCase().includes(searchTerm.toLowerCase()) ||
          key.key.toLowerCase().includes(searchTerm.toLowerCase()),
      )
    : [];

  return (
    <div className="page-shell page-flat">
      <div className="page-header">
        <h1 className="page-title">API Key Management</h1>
        <p className="page-subtitle">Create and revoke service keys for collaborators.</p>
      </div>

      <section className="flat-section">
        <div className="flat-toolbar">
          <Input
            placeholder="Owner ID"
            value={ownerId}
            onChange={(e) => setOwnerId(e.target.value)}
            size="sm"
            classNames={{
              inputWrapper: 'bg-white border-gray-200',
            }}
          />
          <Button
            color="primary"
            onClick={() => generateMutation.mutate(ownerId)}
            isDisabled={!ownerId || generateMutation.isPending}
            isLoading={generateMutation.isPending}
            startContent={<Plus className="h-4 w-4" />}
            className="shrink-0"
          >
            Generate
          </Button>
        </div>
        <div className="mt-3">
          <Input
            placeholder="Search owner or key"
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

      <section className="flat-section flat-grow">
        <Table
          aria-label="API key list"
          removeWrapper
          classNames={{
            th: 'bg-transparent text-gray-700 font-semibold border-b border-black/10',
            td: 'text-gray-900 border-b border-black/5',
          }}
        >
          <TableHeader>
            <TableColumn>Key</TableColumn>
            <TableColumn>Owner</TableColumn>
            <TableColumn>Permissions</TableColumn>
            <TableColumn>Created</TableColumn>
            <TableColumn>Status</TableColumn>
            <TableColumn align="end">Actions</TableColumn>
          </TableHeader>
          <TableBody
            items={filteredKeys || []}
            isLoading={isLoading}
            loadingContent={<Spinner />}
            emptyContent={
              <div className="flat-empty">
                <Key className="mx-auto mb-3 h-10 w-10 opacity-30" />
                <p>No API keys.</p>
              </div>
            }
          >
            {(apiKey) => (
              <TableRow key={apiKey.key} className={apiKey.revoked ? 'opacity-50' : ''}>
                <TableCell>
                  <div className="flex items-center gap-2">
                    <code className="text-xs text-gray-700">{apiKey.key.substring(0, 20)}...</code>
                    <Tooltip content={copiedKey === apiKey.key ? 'Copied' : 'Copy key'}>
                      <Button
                        isIconOnly
                        size="sm"
                        variant="light"
                        onClick={() => copyToClipboard(apiKey.key)}
                      >
                        {copiedKey === apiKey.key ? (
                          <CheckCircle className="h-4 w-4 text-green-600" />
                        ) : (
                          <Copy className="h-4 w-4 text-gray-400" />
                        )}
                      </Button>
                    </Tooltip>
                  </div>
                </TableCell>
                <TableCell>
                  <Chip size="sm" variant="flat" className="bg-gray-100 text-gray-700">
                    {apiKey.owner_id}
                  </Chip>
                </TableCell>
                <TableCell>
                  <div className="flex flex-wrap gap-1">
                    {apiKey.permissions.map((perm) => (
                      <Chip key={perm} size="sm" variant="dot" className="text-gray-600">
                        {perm}
                      </Chip>
                    ))}
                  </div>
                </TableCell>
                <TableCell>
                  <span className="text-sm text-gray-500">{formatDate(apiKey.created_at)}</span>
                </TableCell>
                <TableCell>
                  {apiKey.revoked ? (
                    <Chip size="sm" color="danger" variant="flat">
                      Revoked
                    </Chip>
                  ) : (
                    <Chip size="sm" color="success" variant="flat">
                      Active
                    </Chip>
                  )}
                </TableCell>
                <TableCell>
                  {!apiKey.revoked && (
                    <Button
                      size="sm"
                      color="danger"
                      variant="flat"
                      onClick={() => {
                        if (confirm('Revoke this key?')) {
                          revokeMutation.mutate(apiKey.key);
                        }
                      }}
                      isDisabled={revokeMutation.isPending}
                      startContent={<Trash2 className="h-3 w-3" />}
                    >
                      Revoke
                    </Button>
                  )}
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </section>
    </div>
  );
}
