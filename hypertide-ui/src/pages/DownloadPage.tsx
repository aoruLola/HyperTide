import { useState } from 'react';
import { Input, Button, Card, CardBody } from '@heroui/react';
import { Download, Search } from 'lucide-react';
import { useMutation } from '@tanstack/react-query';
import { apiClient } from '../lib/api';

export function DownloadPage() {
  const [hash, setHash] = useState('');

  const downloadMutation = useMutation({
    mutationFn: async (hash: string) => {
      const res = await apiClient.storage.download(hash);
      // Create download link
      const url = window.URL.createObjectURL(new Blob([res.data]));
      const link = document.createElement('a');
      link.href = url;
      link.setAttribute('download', hash);
      document.body.appendChild(link);
      link.click();
      link.remove();
    },
  });

  return (
    <div className="h-full flex flex-col bg-background p-6">
      <div className="max-w-2xl mx-auto w-full space-y-6">
        <div>
          <h1 className="text-2xl font-bold text-foreground mb-2">下载文件</h1>
          <p className="text-default-500">通过文件哈希下载文件</p>
        </div>

        <Card>
          <CardBody className="gap-4">
            <Input
              label="文件哈希"
              placeholder="输入文件的 BLAKE3 哈希值"
              value={hash}
              onChange={(e) => setHash(e.target.value)}
              startContent={<Search className="w-4 h-4 text-default-400" />}
            />
            <Button
              color="primary"
              onClick={() => downloadMutation.mutate(hash)}
              isDisabled={!hash || downloadMutation.isPending}
              isLoading={downloadMutation.isPending}
              startContent={<Download className="w-4 h-4" />}
            >
              下载
            </Button>
          </CardBody>
        </Card>
      </div>
    </div>
  );
}
