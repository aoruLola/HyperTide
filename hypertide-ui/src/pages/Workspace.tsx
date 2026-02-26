import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Button, Divider, Input, ScrollShadow } from '@heroui/react';
import {
  Folder,
  File,
  Lock,
  Upload as UploadIcon,
  Download,
  ChevronRight,
  ChevronDown,
  Search,
  FileText,
  Image as ImageIcon,
  Film,
  Music,
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

const getFileIcon = (filename: string) => {
  const ext = filename.split('.').pop()?.toLowerCase();
  if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg'].includes(ext || '')) {
    return <ImageIcon className="h-4 w-4 text-blue-500" />;
  }
  if (['mp4', 'avi', 'mov', 'mkv'].includes(ext || '')) {
    return <Film className="h-4 w-4 text-violet-500" />;
  }
  if (['mp3', 'wav', 'ogg', 'flac'].includes(ext || '')) {
    return <Music className="h-4 w-4 text-pink-500" />;
  }
  if (['txt', 'md', 'json', 'xml'].includes(ext || '')) {
    return <FileText className="h-4 w-4 text-emerald-500" />;
  }
  return <File className="h-4 w-4 text-gray-400" />;
};

export function Workspace() {
  const [selectedFile, setSelectedFile] = useState<FileNode | null>(null);
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set(['/']));
  const [searchQuery, setSearchQuery] = useState('');

  const { data: locks } = useQuery({
    queryKey: ['locks'],
    queryFn: async () => {
      const res = await apiClient.locks.list();
      return res.data;
    },
    refetchInterval: 3000,
  });

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
              { path: '/assets/textures/normal.png', name: 'normal.png', type: 'file', size: 3145728 },
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
          { path: '/scripts/config.json', name: 'config.json', type: 'file', size: 2048 },
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
    return locks?.some((lock) => lock.file_path === path);
  };

  const getLockedBy = (path: string) => {
    return locks?.find((lock) => lock.file_path === path)?.owner_id;
  };

  const renderTree = (node: FileNode, level: number = 0) => {
    const isExpanded = expandedFolders.has(node.path);
    const locked = isLocked(node.path);
    const isSelected = selectedFile?.path === node.path;

    if (searchQuery && !node.name.toLowerCase().includes(searchQuery.toLowerCase())) {
      return null;
    }

    return (
      <div key={node.path}>
        <div
          className={`group relative flex cursor-pointer items-center gap-2 rounded-lg px-3 py-1.5 transition-all duration-150 ${
            isSelected ? 'bg-teal-100/70 text-gray-950 font-semibold' : 'text-gray-600 hover:bg-white/70'
          }`}
          style={{ paddingLeft: `${level * 16 + 12}px` }}
          onClick={() => {
            if (node.type === 'folder') toggleFolder(node.path);
            setSelectedFile(node);
          }}
        >
          {isSelected && <div className="absolute bottom-1 top-1 left-1 w-[3px] rounded-full bg-teal-600" />}

          {node.type === 'folder' && (
            <div
              className={`rounded p-0.5 transition-colors ${
                isSelected ? 'text-gray-900' : 'text-gray-400 hover:text-gray-600'
              }`}
              onClick={(e) => {
                e.stopPropagation();
                toggleFolder(node.path);
              }}
            >
              {isExpanded ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}
            </div>
          )}

          <span className="shrink-0">
            {node.type === 'folder' ? (
              <Folder className={`h-4 w-4 ${isSelected ? 'text-teal-700' : 'text-teal-500'}`} />
            ) : (
              getFileIcon(node.name)
            )}
          </span>

          <span className="truncate text-[13px] tracking-tight">{node.name}</span>

          {locked && <Lock className="ml-auto h-3 w-3 text-amber-500/80" />}
        </div>
        {node.type === 'folder' && isExpanded && (
          <div className="mt-0.5">{node.children?.map((child) => renderTree(child, level + 1))}</div>
        )}
      </div>
    );
  };

  return (
    <div className="page-shell page-flat">
      <div className="page-header">
        <h1 className="page-title">Workspace</h1>
        <p className="page-subtitle">Browse assets and inspect metadata.</p>
      </div>

      <section className="flat-section flat-grow">
        <div className="workspace-flat-grid">
          <div className="workspace-tree-pane">
            <Input
              placeholder="Search assets"
              size="sm"
              variant="flat"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              startContent={<Search className="h-3.5 w-3.5 text-gray-400" />}
              classNames={{
                inputWrapper: 'border border-black/[0.08] bg-white',
              }}
            />
            <ScrollShadow className="workspace-tree-scroll mt-3">{renderTree(fileTree)}</ScrollShadow>
          </div>

          <div className="workspace-detail-pane">
            {selectedFile ? (
              <div className="h-full overflow-auto">
                <div className="workspace-file-head">
                  <div className="workspace-file-icon">
                    {selectedFile.type === 'folder' ? (
                      <Folder className="h-8 w-8 text-teal-600/85" />
                    ) : (
                      <div className="scale-[1.4]">{getFileIcon(selectedFile.name)}</div>
                    )}
                  </div>
                  <div className="min-w-0 flex-1">
                    <h2 className="text-lg font-bold text-gray-900">{selectedFile.name}</h2>
                    <div className="mt-1 flex items-center gap-2 text-[11px] font-medium text-gray-500">
                      <span className="uppercase tracking-widest">{selectedFile.type}</span>
                      <Divider orientation="vertical" className="h-2 bg-gray-200" />
                      <code className="truncate">{selectedFile.path}</code>
                    </div>
                  </div>
                </div>

                {selectedFile.type === 'file' && (
                  <div className="workspace-metrics">
                    <div className="workspace-metric-row">
                      <span className="workspace-metric-label">File Size</span>
                      <span className="workspace-metric-value">{formatBytes(selectedFile.size || 0)}</span>
                    </div>
                    <div className="workspace-metric-row">
                      <span className="workspace-metric-label">Status</span>
                      <span className="workspace-metric-value">
                        {isLocked(selectedFile.path)
                          ? `Locked by ${getLockedBy(selectedFile.path)}`
                          : 'Available'}
                      </span>
                    </div>
                  </div>
                )}

                <div className="mt-5 flex flex-wrap gap-2">
                  <Button color="primary" className="rounded-md px-5 font-semibold" startContent={<Lock className="h-4 w-4" />}>
                    Acquire Lock
                  </Button>
                  <Button variant="bordered" className="rounded-md px-5" startContent={<Download className="h-4 w-4" />}>
                    Download
                  </Button>
                  <Button variant="light" className="rounded-md px-5 text-gray-600" startContent={<UploadIcon className="h-4 w-4" />}>
                    Upload Revision
                  </Button>
                </div>
              </div>
            ) : (
              <div className="flat-empty h-full">
                <Folder className="mx-auto mb-3 h-10 w-10 opacity-30" />
                <p>Select an item to view properties.</p>
              </div>
            )}
          </div>
        </div>
      </section>
    </div>
  );
}
