import { useState, useRef } from 'react';
import { useMutation } from '@tanstack/react-query';
import { Button, Progress, Chip } from '@heroui/react';
import { Upload, File, CheckCircle, AlertCircle, X } from 'lucide-react';
import { apiClient } from '../lib/api';
import { formatBytes } from '../lib/utils';

interface UploadItem {
  file: File;
  status: 'pending' | 'uploading' | 'success' | 'error';
  progress: number;
  result?: any;
  error?: string;
}

export function UploadPage() {
  const [uploads, setUploads] = useState<UploadItem[]>([]);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const uploadMutation = useMutation({
    mutationFn: async (item: UploadItem) => {
      const res = await apiClient.storage.upload(item.file, item.file.name);
      return res.data;
    },
  });

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(e.target.files || []);
    const newUploads = files.map((file) => ({
      file,
      status: 'pending' as const,
      progress: 0,
    }));
    setUploads((prev) => [...prev, ...newUploads]);
    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }
  };

  const handleUpload = async (index: number) => {
    const item = uploads[index];
    setUploads((prev) =>
      prev.map((u, i) => (i === index ? { ...u, status: 'uploading' } : u)),
    );

    try {
      const result = await uploadMutation.mutateAsync(item);
      setUploads((prev) =>
        prev.map((u, i) =>
          i === index ? { ...u, status: 'success', progress: 100, result } : u,
        ),
      );
    } catch (error: any) {
      setUploads((prev) =>
        prev.map((u, i) =>
          i === index
            ? {
                ...u,
                status: 'error',
                error: error.response?.data?.error || 'Upload failed',
              }
            : u,
        ),
      );
    }
  };

  const handleUploadAll = () => {
    uploads.forEach((item, index) => {
      if (item.status === 'pending') {
        handleUpload(index);
      }
    });
  };

  const removeUpload = (index: number) => {
    setUploads((prev) => prev.filter((_, i) => i !== index));
  };

  const pendingCount = uploads.filter((u) => u.status === 'pending').length;

  return (
    <div className="page-shell page-flat">
      <div className="page-header">
        <h1 className="page-title">File Upload</h1>
        <p className="page-subtitle">Upload assets into content-addressable storage.</p>
      </div>

      <section className="flat-section">
        <div className="flat-toolbar">
          <input
            ref={fileInputRef}
            type="file"
            multiple
            onChange={handleFileSelect}
            className="hidden"
            id="file-upload"
          />
          <Button
            as="label"
            htmlFor="file-upload"
            variant="flat"
            className="border border-gray-200 bg-white"
            startContent={<Upload className="h-4 w-4" />}
          >
            Select Files
          </Button>
          {pendingCount > 0 && (
            <Button
              color="primary"
              onClick={handleUploadAll}
              startContent={<Upload className="h-4 w-4" />}
            >
              Upload All ({pendingCount})
            </Button>
          )}
        </div>
      </section>

      <section className="flat-section flat-grow">
        {uploads.length === 0 ? (
          <div className="flat-empty">
            <Upload className="mx-auto mb-4 h-10 w-10 text-gray-400" />
            <p className="mb-1 text-base text-gray-700">Select files to start uploading</p>
            <p className="text-sm text-gray-500">Batch upload is supported.</p>
          </div>
        ) : (
          <div className="flat-list" role="list">
            {uploads.map((item, index) => (
              <div key={index} className="flat-list-item" role="listitem">
                <div className="flex min-w-0 flex-1 items-start gap-3">
                  <div className="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-gray-100">
                    <File className="h-4 w-4 text-gray-500" />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="mb-1 flex items-center justify-between gap-2">
                      <span className="truncate text-sm font-semibold text-gray-900">{item.file.name}</span>
                      <Button isIconOnly size="sm" variant="light" onClick={() => removeUpload(index)}>
                        <X className="h-4 w-4 text-gray-400" />
                      </Button>
                    </div>
                    <div className="mb-2 text-xs text-gray-500">{formatBytes(item.file.size)}</div>

                    {item.status === 'pending' && (
                      <Button size="sm" color="primary" variant="flat" onClick={() => handleUpload(index)}>
                        Start Upload
                      </Button>
                    )}

                    {item.status === 'uploading' && (
                      <div className="space-y-2">
                        <Chip size="sm" color="primary" variant="flat">
                          Uploading...
                        </Chip>
                        <Progress size="sm" isIndeterminate color="primary" />
                      </div>
                    )}

                    {item.status === 'success' && (
                      <div className="space-y-2">
                        <Chip
                          size="sm"
                          color="success"
                          variant="flat"
                          startContent={<CheckCircle className="h-3 w-3" />}
                        >
                          Upload Complete
                        </Chip>
                        {item.result && (
                          <div className="text-xs text-gray-500">
                            <code className="rounded bg-gray-100 px-2 py-1 text-gray-700">
                              {item.result.hash}
                            </code>
                          </div>
                        )}
                      </div>
                    )}

                    {item.status === 'error' && (
                      <Chip
                        size="sm"
                        color="danger"
                        variant="flat"
                        startContent={<AlertCircle className="h-3 w-3" />}
                      >
                        {item.error}
                      </Chip>
                    )}
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
