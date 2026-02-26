import { NavLink } from 'react-router-dom';
import { 
  FolderTree, 
  Lock, 
  Upload, 
  Download, 
  Key, 
  History,
  Search
} from 'lucide-react';

interface SidebarProps {
  width: number;
  onResize: (width: number) => void;
}

export function Sidebar({ width }: SidebarProps) {
  const navItems = [
    { path: '/', icon: FolderTree, label: '工作区' },
    { path: '/locks', icon: Lock, label: '锁定管理' },
    { path: '/upload', icon: Upload, label: '上传文件' },
    { path: '/download', icon: Download, label: '下载管理' },
    { path: '/search', icon: Search, label: '搜索文件' },
    { path: '/history', icon: History, label: '操作历史' },
    { path: '/keys', icon: Key, label: '密钥管理' },
  ];

  return (
    <aside
      style={{ width: `${width}px` }}
      className="bg-gray-950 border-r border-gray-800 flex flex-col"
    >
      {/* Navigation */}
      <nav className="flex-1 py-2">
        {navItems.map((item) => (
          <NavLink
            key={item.path}
            to={item.path}
            className={({ isActive }) =>
              `flex items-center gap-3 px-4 py-2.5 text-sm transition-colors ${
                isActive
                  ? 'bg-purple-600/20 text-purple-400 border-l-2 border-purple-500'
                  : 'text-gray-400 hover:bg-gray-800 hover:text-gray-300'
              }`
            }
          >
            <item.icon className="w-4 h-4" />
            <span>{item.label}</span>
          </NavLink>
        ))}
      </nav>

      {/* Bottom Info */}
      <div className="p-4 border-t border-gray-800 text-xs text-gray-500">
        <div>CARP Component</div>
        <div className="mt-1">Designed by Lyura</div>
      </div>
    </aside>
  );
}
