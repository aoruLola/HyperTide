import { useState } from 'react';
import { Input, Button } from '@heroui/react';
import { Download, Search } from 'lucide-react';
import { useMutation } from '@tanstack/react-query';
import { apiClient } from '../lib/api';

export function DownloadPage() {
  const [hash, setHash] = useState('');

  const downloadMutation = useMutation({
    mutationFn: async (value: string) => {
      const res = await apiClient.storage.download(value);
      const url = window.URL.createObjectURL(new Blob([res.data]));
      const link = document.createElement('a');
      link.href = url;
      link.setAttribute('download', value);
      document.body.appendChild(link);
      link.click();
      link.remove();
    },
  });

  return (
    <div className="page-shell page-flat">
      <div className="page-header">
        <h1 className="page-title">Download File</h1>
        <p className="page-subtitle">Retrieve assets from storage by BLAKE3 hash.</p>
      </div>

      <section className="flat-section max-w-2xl">
        <div className="mb-3 flex items-center gap-2 text-sm font-semibold text-gray-700">
          <Download className="h-4 w-4" />
          <span>Download by Hash</span>
        </div>
        <div className="flat-toolbar">
          <Input
            label="File Hash"
            placeholder="Enter BLAKE3 hash"
            value={hash}
            onChange={(e) => setHash(e.target.value)}
            startContent={<Search className="h-4 w-4 text-gray-400" />}
            classNames={{
              label: 'font-medium text-gray-700',
              inputWrapper: 'bg-white border-gray-200',
            }}
          />
          <Button
            color="primary"
            onClick={() => downloadMutation.mutate(hash)}
            isDisabled={!hash || downloadMutation.isPending}
            isLoading={downloadMutation.isPending}
            startContent={<Download className="h-4 w-4" />}
          >
            Download
          </Button>
        </div>
      </section>
    </div>
  );
}
