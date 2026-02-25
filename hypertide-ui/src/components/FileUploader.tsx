import { useState, useRef } from 'react';
import { useMutation } from '@tanstack/react-query';
import { Upload, File, CheckCircle, AlertCircle, Loader2 } from 'lucide-react';
import { apiClient } from '../lib/api';
import { formatBytes } from '../lib/utils';

export function FileUploader() {
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [uploadResult, setUploadResult] = useState<any>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const uploadMutation = useMutation({
    mutationFn: async (file: File) => {
      const res = await apiClient.storage.upload(file, file.name);
      return res.data;
    },
    onSuccess: (data) => {
      setUploadResult(data);
      setSelectedFile(null);
      if (fileInputRef.current) {
        fileInputRef.current.value = '';
      }
    },
  });

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      setSelectedFile(file);
      setUploadResult(null);
    }
  };

  const handleUpload = () => {
    if (selectedFile) {
      uploadMutation.mutate(selectedFile);
    }
  };

  return (
    <div className="space-y-6">
      {/* Upload Area */}
      <div className="bg-black/30 backdrop-blur-sm rounded-lg border border-purple-500/20 p-6">
        <h2 className="text-xl font-semibold text-white mb-4">上传文件</h2>
        
        <div className="border-2 border-dashed border-purple-500/30 rounded-lg p-8 text-center hover:border-purple-500/50 transition-colors">
          <input
            ref={fileInputRef}
            type="file"
            onChange={handleFileSelect}
            className="hidden"
            id="file-upload"
          />
          <label
            htmlFor="file-upload"
            className="cursor-pointer flex flex-col items-center gap-3"
          >
            <div className="w-16 h-16 bg-purple-500/20 rounded-full flex items-center justify-center">
              <Upload className="w-8 h-8 text-purple-400" />
            </div>
            <div className="text-white font-medium">点击选择文件或拖拽到此处</div>
            <div className="text-sm text-gray-400">支持所有文件类型</div>
          </label>
        </div>

        {selectedFile && (
          <div className="mt-4 bg-black/40 rounded-lg p-4 border border-purple-500/10">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <File className="w-5 h-5 text-purple-400" />
                <div>
                  <div className="text-white font-medium">{selectedFile.name}</div>
                  <div className="text-sm text-gray-400">{formatBytes(selectedFile.size)}</div>
                </div>
              </div>
              <button
                onClick={handleUpload}
                disabled={uploadMutation.isPending}
                className="px-6 py-2 bg-purple-600 hover:bg-purple-700 disabled:bg-gray-600 disabled:cursor-not-allowed text-white rounded-lg font-medium transition-colors flex items-center gap-2"
              >
                {uploadMutation.isPending ? (
                  <>
                    <Loader2 className="w-4 h-4 animate-spin" />
                    上传中...
                  </>
                ) : (
                  <>
                    <Upload className="w-4 h-4" />
                    上传
                  </>
                )}
              </button>
            </div>
          </div>
        )}

        {uploadMutation.isError && (
          <div className="mt-4 flex items-center gap-2 text-red-400 text-sm bg-red-500/10 rounded-lg p-3 border border-red-500/20">
            <AlertCircle className="w-4 h-4" />
            <span>{(uploadMutation.error as any)?.response?.data?.error || '上传失败'}</span>
          </div>
        )}
      </div>

      {/* Upload Result */}
      {uploadResult && (
        <div className="bg-black/30 backdrop-blur-sm rounded-lg border border-green-500/20 p-6">
          <div className="flex items-start gap-3">
            <CheckCircle className="w-6 h-6 text-green-400 flex-shrink-0 mt-0.5" />
            <div className="flex-1">
              <h3 className="text-lg font-semibold text-white mb-3">上传成功</h3>
              <div className="space-y-2 text-sm">
                <div className="flex">
                  <span className="text-gray-400 w-24">文件哈希:</span>
                  <span className="text-white font-mono">{uploadResult.hash}</span>
                </div>
                <div className="flex">
                  <span className="text-gray-400 w-24">原始路径:</span>
                  <span className="text-white">{uploadResult.original_path}</span>
                </div>
                <div className="flex">
                  <span className="text-gray-400 w-24">文件大小:</span>
                  <span className="text-white">{formatBytes(uploadResult.size_bytes)}</span>
                </div>
                <div className="flex">
                  <span className="text-gray-400 w-24">存储时间:</span>
                  <span className="text-white">{new Date(uploadResult.stored_at).toLocaleString('zh-CN')}</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
