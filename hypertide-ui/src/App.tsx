import { useState } from 'react';
import { Lock, Upload, Key, FileText } from 'lucide-react';
import { LockManager } from './components/LockManager';
import { FileUploader } from './components/FileUploader';
import { KeyManager } from './components/KeyManager';
import { useAppStore } from './store/useAppStore';

type Tab = 'locks' | 'upload' | 'keys';

function App() {
  const [activeTab, setActiveTab] = useState<Tab>('locks');
  const { userId } = useAppStore();

  return (
    <div className="min-h-screen bg-gradient-to-br from-gray-900 via-purple-900 to-gray-900">
      {/* Header */}
      <header className="bg-black/30 backdrop-blur-sm border-b border-purple-500/20">
        <div className="container mx-auto px-4 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="text-3xl font-bold bg-gradient-to-r from-red-500 via-purple-500 to-purple-600 bg-clip-text text-transparent">
                HYPERTIDE
              </div>
              <div className="text-xs text-gray-400">Surface 26.0.1 Preview</div>
            </div>
            <div className="flex items-center gap-2 text-sm text-gray-300">
              <FileText className="w-4 h-4" />
              <span>User: {userId}</span>
            </div>
          </div>
        </div>
      </header>

      {/* Navigation */}
      <nav className="bg-black/20 backdrop-blur-sm border-b border-purple-500/10">
        <div className="container mx-auto px-4">
          <div className="flex gap-1">
            <TabButton
              active={activeTab === 'locks'}
              onClick={() => setActiveTab('locks')}
              icon={<Lock className="w-4 h-4" />}
              label="文件锁定"
            />
            <TabButton
              active={activeTab === 'upload'}
              onClick={() => setActiveTab('upload')}
              icon={<Upload className="w-4 h-4" />}
              label="文件上传"
            />
            <TabButton
              active={activeTab === 'keys'}
              onClick={() => setActiveTab('keys')}
              icon={<Key className="w-4 h-4" />}
              label="密钥管理"
            />
          </div>
        </div>
      </nav>

      {/* Content */}
      <main className="container mx-auto px-4 py-6">
        {activeTab === 'locks' && <LockManager />}
        {activeTab === 'upload' && <FileUploader />}
        {activeTab === 'keys' && <KeyManager />}
      </main>
    </div>
  );
}

interface TabButtonProps {
  active: boolean;
  onClick: () => void;
  icon: React.ReactNode;
  label: string;
}

function TabButton({ active, onClick, icon, label }: TabButtonProps) {
  return (
    <button
      onClick={onClick}
      className={`
        flex items-center gap-2 px-4 py-3 font-medium transition-all
        ${
          active
            ? 'text-purple-400 border-b-2 border-purple-400 bg-purple-500/10'
            : 'text-gray-400 hover:text-gray-300 hover:bg-white/5'
        }
      `}
    >
      {icon}
      <span>{label}</span>
    </button>
  );
}

export default App;
