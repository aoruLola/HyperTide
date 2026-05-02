import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Key, Plus, Trash2, Loader2, Copy, CheckCircle } from 'lucide-react';
import { apiClient } from '../lib/api';
import { formatDate } from '../lib/utils';

export function KeyManager() {
  const [ownerId, setOwnerId] = useState('');
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

  return (
    <div className="space-y-6">
      {/* Generate Key Form */}
      <div className="bg-black/30 backdrop-blur-sm rounded-lg border border-purple-500/20 p-6">
        <h2 className="text-xl font-semibold text-white mb-4">生成 API Key</h2>
        <div className="flex gap-3">
          <input
            type="text"
            value={ownerId}
            onChange={(e) => setOwnerId(e.target.value)}
            placeholder="输入所有者 ID，例如: alice"
            className="flex-1 px-4 py-2 bg-black/50 border border-purple-500/30 rounded-lg text-white placeholder-gray-500 focus:outline-none focus:border-purple-500"
          />
          <button
            onClick={() => generateMutation.mutate(ownerId)}
            disabled={!ownerId || generateMutation.isPending}
            className="px-6 py-2 bg-purple-600 hover:bg-purple-700 disabled:bg-gray-600 disabled:cursor-not-allowed text-white rounded-lg font-medium transition-colors flex items-center gap-2"
          >
            {generateMutation.isPending ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <Plus className="w-4 h-4" />
            )}
            生成
          </button>
        </div>
      </div>

      {/* Keys List */}
      <div className="bg-black/30 backdrop-blur-sm rounded-lg border border-purple-500/20 p-6">
        <h2 className="text-xl font-semibold text-white mb-4">API Keys</h2>
        {isLoading ? (
          <div className="flex items-center justify-center py-8">
            <Loader2 className="w-6 h-6 animate-spin text-purple-400" />
          </div>
        ) : keys && keys.length > 0 ? (
          <div className="space-y-3">
            {keys.map((apiKey) => (
              <div
                key={apiKey.key}
                className={`bg-black/40 rounded-lg p-4 border transition-colors ${
                  apiKey.revoked
                    ? 'border-red-500/20 opacity-60'
                    : 'border-purple-500/10 hover:border-purple-500/30'
                }`}
              >
                <div className="flex items-start justify-between gap-4">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-2">
                      <Key className="w-4 h-4 text-purple-400 flex-shrink-0" />
                      <code className="text-white font-mono text-sm truncate">
                        {apiKey.key}
                      </code>
                      <button
                        onClick={() => copyToClipboard(apiKey.key)}
                        className="text-gray-400 hover:text-white transition-colors"
                        title="复制"
                      >
                        {copiedKey === apiKey.key ? (
                          <CheckCircle className="w-4 h-4 text-green-400" />
                        ) : (
                          <Copy className="w-4 h-4" />
                        )}
                      </button>
                    </div>
                    <div className="text-sm text-gray-400 space-y-1">
                      <div>所有者: {apiKey.owner_id}</div>
                      <div>
                        权限: {apiKey.permissions.join(', ')}
                      </div>
                      <div>创建时间: {formatDate(apiKey.created_at)}</div>
                      {apiKey.expires_at && (
                        <div>过期时间: {formatDate(apiKey.expires_at)}</div>
                      )}
                      {apiKey.revoked && (
                        <div className="text-red-400 font-medium">已撤销</div>
                      )}
                    </div>
                  </div>
                  {!apiKey.revoked && (
                    <button
                      onClick={() => revokeMutation.mutate(apiKey.key)}
                      disabled={revokeMutation.isPending}
                      className="px-3 py-1.5 bg-red-600 hover:bg-red-700 disabled:bg-gray-600 text-white text-sm rounded-md transition-colors flex items-center gap-1.5 flex-shrink-0"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                      撤销
                    </button>
                  )}
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-center py-8 text-gray-400">暂无 API Keys</div>
        )}
      </div>
    </div>
  );
}
