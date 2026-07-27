import { useState, useCallback } from 'react';
import { Search, Loader2 } from 'lucide-react';
import { queryRagSandbox, SearchResult } from '@/services/ragService';

export const SearchPanel = () => {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchResult[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [searched, setSearched] = useState(false);

  const handleSearch = useCallback(async () => {
    const trimmed = query.trim();
    if (!trimmed) return;
    setLoading(true);
    setSearched(true);
    try {
      const res = await queryRagSandbox(trimmed);
      setResults(res);
    } catch (err) {
      console.error('[SearchPanel] Query failed:', err);
      setResults([]);
    } finally {
      setLoading(false);
    }
  }, [query]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') handleSearch();
  };

  return (
    <div className="bg-offline-surface-dark border border-offline-border rounded-xl p-4 flex flex-col gap-3">
      <span className="text-[10px] font-mono text-offline-core uppercase tracking-widest font-bold">
        Semantic Search
      </span>

      <div className="flex gap-2">
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Search indexed documents..."
          className="flex-1 bg-white/[0.02] hover:bg-white/[0.04] border border-white/10 rounded-lg px-3 py-2 text-xs font-mono text-primary-txt focus:outline-none focus:border-offline-core/50 focus:ring-1 focus:ring-offline-core/30 transition-all duration-300"
        />
        <button
          onClick={handleSearch}
          disabled={loading || !query.trim()}
          className="px-3 py-2 bg-offline-core/10 border border-offline-core/30 rounded-lg text-offline-core hover:bg-offline-core/20 transition-colors disabled:opacity-50 cursor-pointer"
        >
          {loading ? <Loader2 size={14} className="animate-spin" /> : <Search size={14} />}
        </button>
      </div>

      <div className="flex flex-col gap-2 min-h-0 max-h-[320px] overflow-y-auto custom-scrollbar">
        {loading && (
          <div className="flex items-center justify-center py-8 text-secondary-txt/60">
            <Loader2 size={16} className="animate-spin mr-2" />
            <span className="text-[10px] font-mono">Searching...</span>
          </div>
        )}

        {!loading && !searched && (
          <div className="flex items-center justify-center py-8 text-tertiary-txt/50">
            <span className="text-[10px] font-mono">Enter a query to search indexed documents.</span>
          </div>
        )}

        {!loading && searched && results && results.length === 0 && (
          <div className="flex items-center justify-center py-8 text-tertiary-txt/50">
            <span className="text-[10px] font-mono">No results found</span>
          </div>
        )}

        {!loading && results && results.map((r, i) => (
          <div
            key={i}
            className="bg-white/[0.02] border border-white/5 rounded-lg p-3 space-y-1.5 hover:border-offline-core/20 transition-colors"
          >
            <div className="flex items-center justify-between gap-2">
              <span className="text-[10px] font-mono text-offline-core/80 truncate flex-1" title={r.note}>
                {r.note}
              </span>
              <span className="text-[9px] font-mono text-offline-core font-bold shrink-0">
                {(r.score * 100).toFixed(1)}%
              </span>
            </div>
            <div className="w-full h-1 bg-white/5 rounded-full overflow-hidden">
              <div
                className="h-full bg-offline-core rounded-full transition-all duration-300"
                style={{ width: `${(r.score * 100).toFixed(1)}%` }}
              />
            </div>
            <p className="text-[10px] font-sans text-secondary-txt/70 leading-relaxed line-clamp-3">
              {r.content}
            </p>
          </div>
        ))}
      </div>
    </div>
  );
};
