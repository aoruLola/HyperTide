import { History } from 'lucide-react';

export function HistoryPage() {
  return (
    <div className="page-shell page-flat">
      <div className="page-header">
        <h1 className="page-title">Operation History</h1>
        <p className="page-subtitle">Review all recent actions across the workspace.</p>
      </div>

      <section className="flat-section">
        <div className="flat-empty">
          <History className="mx-auto mb-3 h-10 w-10 opacity-50" />
          <p>History features are under active development.</p>
        </div>
      </section>
    </div>
  );
}
