import { Card, CardBody } from '@heroui/react';
import { History } from 'lucide-react';

export function HistoryPage() {
  return (
    <div className="h-full flex flex-col bg-background p-6">
      <div className="max-w-4xl mx-auto w-full space-y-6">
        <div>
          <h1 className="text-2xl font-bold text-foreground mb-2">操作历史</h1>
          <p className="text-default-500">查看所有操作记录</p>
        </div>

        <Card>
          <CardBody>
            <div className="text-center text-default-400 py-12">
              <History className="w-16 h-16 mx-auto mb-4 opacity-50" />
              <p>功能开发中...</p>
            </div>
          </CardBody>
        </Card>
      </div>
    </div>
  );
}
