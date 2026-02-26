import { useState, useRef } from 'react';
import { useMutation } from '@tanstack/react-query';
import {
  Button,
  Card,
  CardBody,
  Progress,
  Chip,
} from '@heroui/react';
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
    const newUploads = files.map(file => ({
      file,
      status: 'pending' as const,
      progress: 0,
    }));
    setUploads(prev => [...prev, ...newUploads]);
    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }
  });

  const handleUpload = async (index: number) => {
    const item = uploads[index];
    setUploads(prev => prev.map((u, i) => i === index ? { ...u, status: 'uploading' } : u));

    try {
      const result = await uploadMutation.mutateAsync(item);
      setUploads(prev => prev.map((u, i) => 
        i === index ? { ...u, status: 'success', progress: 100, result } : u
      ));
    } catch (error: any) {
      setUploads(prev => prev.map((u, i) => 
        i === index ? { ...u, status: 'error', error: error.response?.data?.error || '上传失败' } : u
      ));
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
    setUploads(prev => prev.filter((_, i) => i !== index));
  };

  const pendingCount = uploads.filter(u => u.status === 'pending').length;

  return (
    <div className="h-full flex flex-col bg-background p-6 gap-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-foreground mb-2">上传文件</h1>
          <p className="text-default-500">上传文件到内容寻址存储系统</p>
        </div>
        <div className="flex gap-2">
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
            startContent={<Upload className="w-4 h-4" />}
          >
            选择文件
          </Button>
          {pendingCount > 0 && (
            <Button
              color="primary"
              onClick={handleUploadAll}
              startContent={<Upload className="w-4 h-4" />}
            >
              上传全部 ({pendingCount})
            </Button>
          )}
        </div>
      </div>

      {/* Upload List */}
      <div className="flex-1 overflow-auto">
        {uploads.length === 0 ? (
          <Card className="h-full">
            <CardBody className="flex items-center justify-center">
              <div className="text-center text-default-400">
                <Upload className="w-16 h-16 mx-auto mb-4 opacity-50" />
                <p className="text-lg mb-2">选择文件开始上传</p>
                <p className="text-sm">支持批量上传</p>
              </div>
            </CardBody>
          </Card>
        ) : (
          <div className="space-y-3">
            {uploads.map((item, index) => (
              <Card key={index}>
                <CardBody>
                  <div className="flex items-start gap-3">
                    <File className="w-5 h-5 text-default-400 flex-shrink-0 mt-1" />
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center justify-between mb-2">
                        <span className="text-sm font-medium truncate">
                          {item.file.name}
                        </span>
                        <Button
                          isIconOnly
                          size="sm"
                          variant="light"
                          onClick={() => removeUpload(index)}
                        >
                          <X className="w-4 h-4" />
                        </Button>
                      </div>
                      <div className="text-xs text-default-400 mb-3">
                        {formatBytes(item.file.size)}
                      </div>

                      {/* Status */}
                      {item.status === 'pending' && (
                        <Button
                          size="sm"
                          color="primary"
                          variant="flat"
                          onClick={() => handleUpload(index)}
                        >
                          开始上传
                        </Button>
                      )}

                      {item.status === 'uploading' && (
                        <div className="space-y-2">
                          <Chip size="sm" color="primary" variant="flat">上传中...</Chip>
                          <Progress size="sm" isIndeterminate color="primary" />
                        </div>
                      )}

                      {item.status === 'success' && (
                        <div className="space-y-2">
                          <Chip
                            size="sm"
                            color="success"
                            variant="flat"
                            startContent={<CheckCircle className="w-3 h-3" />}
                          >
                            上传成功
                          </Chip>
                          {item.result && (
                            <div className="text-xs text-default-400">
                              <code className="bg-default-100 px-2 py-1 rounded">
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
                          startContent={<AlertCircle className="w-3 h-3" />}
                        >
                          {item.error}
                        </Chip>
                      )}
                    </div>
                  </div>
                </CardBody>
              </Card>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
