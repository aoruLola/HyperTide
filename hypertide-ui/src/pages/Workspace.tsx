import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import {
  Card,
  CardBody,
  Button,
  Chip,
  Divider,
} from '@heroui/react';
import { 
  Folder, 
  File, 
  Lock, 
  Upload as UploadIcon,
  Download,
  ChevronRight,
  ChevronDown,
} from 'lucide-react';
import { apiClient } from '../lib/api';
import { formatBytes } from '../lib/utils';

interface FileNode {
  path: string;
  name: string;
  type: 'file' | 'folder';
  hash?: string;
  size?: number;
  locked?: boolean;
  lockedBy?: string;
  children?: FileNode[];
}

export function Workspace() {
  const [selectedFile, setSelectedFile] = useState<FileNode | null>(null);
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set(['/']));

  const { data: locks } = useQuery({
    queryKey: ['locks'],
    queryFn: async () => {
      const res = await apiClient.locks.list();
      return res.data;
    },
    refetchInterval: 3000,
  });

  // Mock file tree
  const fileTree: FileNode = {
    path: '/',
    name: 'Root',
    type: 'folder',
    children: [
      {
        path: '/assets',
        name: 'assets',
        type: 'folder',
        children: [
          {
            path: '/assets/models',
            name: 'models',
            type: 'folder',
            children: [
              { path: '/assets/models/character.fbx', name: 'character.fbx', type: 'file', size: 2048000 },
              { path: '/assets/models/weapon.fbx', name: 'weapon.fbx', type: 'file', size: 512000 },
            ],
          },
          {
            path: '/assets/textures',
            name: 'textures',
            type: 'folder',
            children: [
              { path: '/assets/textures/diffuse.png', name: 'diffuse.png', type: 'file', size: 4096000 },
            ],
          },
        ],
      },
      {
        path: '/scripts',
        name: 'scripts',
        type: 'folder',
        children: [
          { path: '/scripts/main.lua', name: 'main.lua', type: 'file', size: 8192 },
        ],
      },
    ],
  };

  const toggleFolder = (path: string) => {
    const newExpanded = new Set(expandedFolders);
    if (newExpanded.has(path)) {
      newExpanded.delete(path);
    } else {
      newExpanded.add(path);
    }
    setExpandedFolders(newExpanded);
  };

  const isLocked = (path: string) => {
    return locks?.some(lock => lock.file_path === path);
  };

  const getLockedBy = (path: string) => {
    return locks?.find(lock => lock.file_path === path)?.owner_id;
  };

  const renderTree = (node: FileNode, level: number = 0) => {
    const isExpanded = expandedFolders.has(node.path);
    const locked = isLocked(node.path);
    const lockedBy = getLockedBy(node.path);

    return (
      <div key={node.path}>
        <div
          className={`flex items-center gap-2 px-2 py-1.5 hover:bg-default-100 cursor-pointer rounded-md ${
            selectedFile?.path === node.path ? 'bg-default-100' : ''
          }`}
          style={{ paddingLeft: `${level * 16 + 8}px` }}
          onClick={() => {
            if (node.type === 'folder') {
              toggleFolder(node.path);
            }
            setSelectedFile(node);
          }}
        >
          {node.type === 'folder' && (
            <button onClick={(e) => { e.stopPropagation(); toggleFolder(node.path); }}>
              {isExpanded ? (
                <ChevronDown className="w-4 h-4 text-default-400" />
              ) : (
                <ChevronRight className="w-4 h-4 text-default-400" />
              )}
            </button>
          )}
          {node.type === 'folder' ? (
            <Folder className="w-4 h-4 text-primary" />
          ) : (
            <File className="w-4 h-4 text-default-400" />
          )}
          <span className="text-sm flex-1">{node.name}</span>
          {locked && <Lock className="w-3 h-3 text-warning" title={`锁定者: ${lockedBy}`} />}
        </div>
        {node.type === 'folder' && isExpanded && node.children?.map(child => renderTree(child, level + 1))}
      </div>
    );
  };

  return (
    <div className="h-full flex bg-background">
      {/* Left: File Tree */}
      <div className="w-80 border-r border-divider overflow-y-auto p-4">
        <h2 className="text-sm font-semibold mb-3">文件树</h2>
        <div>
          {renderTree(fileTree)}
        </div>
      </div>

      {/* Right: File Details */}
      <div className="flex-1 p-6">
        {selectedFile ? (
          <div className="max-w-2xl">
            <Card>
              <CardBody className="gap-4">
                {/* Header */}
                <div className="flex items-center gap-3">
                  {selectedFile.type === 'folder' ? (
                    <Folder className="w-8 h-8 text-primary" />
                  ) : (
                    <File className="w-8 h-8 text-default-400" />
                  )}
                  <div className="flex-1">
                    <h2 className="text-xl font-semibold">{selectedFile.name}</h2>
                    <p className="text-sm text-default-500">{selectedFile.path}</p>
                  </div>
                </div>

                {selectedFile.type === 'file' && (
                  <>
                    <Divider />
                    
                    {/* File Info */}
                    <div className="space-y-2">
                      <h3 className="text-sm font-semibold">文件信息</h3>
                      {selectedFile.size && (
                        <div className="flex justify-between text-sm">
                          <span className="text-default-500">大小</span>
                          <span>{formatBytes(selectedFile.size)}</span>
                        </div>
                      )}
                      <div className="flex justify-between text-sm">
                        <span className="text-default-500">状态</span>
                        {isLocked(selectedFile.path) ? (
                          <Chip size="sm" color="warning" variant="flat">
                            已锁定 ({getLockedBy(selectedFile.path)})
                          </Chip>
                        ) : (
                          <Chip size="sm" color="success" variant="flat">
                            可用
                          </Chip>
                        )}
                      </div>
                    </div>

                    <Divider />

                    {/* Actions */}
                    <div className="space-y-2">
                      <h3 className="text-sm font-semibold mb-2">操作</h3>
                      <Button
                        fullWidth
                        color="primary"
                        variant="flat"
                        startContent={<Lock className="w-4 h-4" />}
                      >
                        锁定文件
                      </Button>
                      <Button
                        fullWidth
                        variant="flat"
                        startContent={<Download className="w-4 h-4" />}
                      >
                        下载文件
                      </Button>
                      <Button
                        fullWidth
                        variant="flat"
                        startContent={<UploadIcon className="w-4 h-4" />}
                      >
                        上传新版本
                      </Button>
                    </div>
                  </>
                )}
              </CardBody>
            </Card>
          </div>
        ) : (
          <div className="h-full flex items-center justify-center">
            <div className="text-center text-default-400">
              <Folder className="w-16 h-16 mx-auto mb-4 opacity-50" />
              <p>从左侧选择一个文件或文件夹</p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
