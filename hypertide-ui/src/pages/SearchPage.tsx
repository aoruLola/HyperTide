import { useState } from 'react';
import { Input } from '@heroui/react';
import { Search } from 'lucide-react';

export function SearchPage() {
  const [searchTerm, setSearchTerm] = useState('');

  return (
    <div className="page-shell page-flat">
      <div className="page-header">
        <h1 className="page-title">Search Files</h1>
        <p className="page-subtitle">Find assets by path or hash.</p>
      </div>

      <section className="flat-section max-w-3xl">
        <Input
          label="Search"
          placeholder="Enter file path or hash"
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          startContent={<Search className="h-4 w-4 text-default-400" />}
          classNames={{
            inputWrapper: 'bg-white border-gray-200',
          }}
        />
      </section>

      <section className="flat-section">
        <div className="flat-empty">
          Search features are under active development.
        </div>
      </section>
    </div>
  );
}
