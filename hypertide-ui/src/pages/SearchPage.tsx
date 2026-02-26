import { useState } from 'react';
import { Input, Card, CardBody } from '@heroui/react';
import { Search } from 'lucide-react';

export function SearchPage() {
  const [searchTerm, setSearchTerm] = useState('');

  return (
    <div className="h-full flex flex-col bg-background p-6">
      <div className="max-w-4xl mx-auto w-full space-y-6">
        <div>
          <h1 className="text-2xl font-bold text-foreground mb-2">搜索文件</h1>
          <p className="text-default-500">按路径或哈希搜索文件</p>
        </div>

        <Card>
          <CardBody>
            <Input
              label="搜索"
              placeholder="输入文件路径或哈希..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              startContent={<Search className="w-4 h-4 text-default-400" />}
            />
          </CardBody>
        </Card>

        <div className="text-center text-default-400 py-12">
          功能开发中...
        </div>
      </div>
    </div>
  );
}
