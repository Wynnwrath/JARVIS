import { useState, useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { ChevronDown, ChevronRight, Library } from 'lucide-react';

interface SourcesBlockProps {
  content: string;
  theme: 'online' | 'offline';
}

interface Hit {
  number: number;
  score: number;
  source: string;
  chunk: number;
  snippet: string;
}

export const SourcesBlock = ({ content, theme }: SourcesBlockProps) => {
  const [isOpen, setIsOpen] = useState(false);

  const hits = useMemo(() => {
    const results: Hit[] = [];
    const regex = /--- hit (\d+) \(score: ([\d.]+)\) ---\n\[source: (.+?) \| chunk: (\d+)\]\n([\s\S]*?)(?=\n--- hit|\s*$)/g;
    let match: RegExpExecArray | null;
    while ((match = regex.exec(content)) !== null) {
      results.push({
        number: parseInt(match[1], 10),
        score: parseFloat(match[2]),
        source: match[3],
        chunk: parseInt(match[4], 10),
        snippet: match[5].trim(),
      });
    }
    return results;
  }, [content]);

  if (hits.length === 0) return null;

  const accentClass = theme === 'online' ? 'text-theme-accent' : 'text-offline-core';
  const borderClass = theme === 'online' ? 'border-theme-border' : 'border-offline-border';
  const bgClass = theme === 'online' ? 'bg-theme-surface-2' : 'bg-offline-surface-dark';

  return (
    <div className={`border ${borderClass}/30 ${bgClass}/30 rounded-lg overflow-hidden text-[12px] my-2`}>
      <button
        onClick={() => setIsOpen(!isOpen)}
        className={`w-full flex items-center gap-2 px-3 py-1.5 ${bgClass}/20 ${accentClass}/60 hover:${accentClass} select-none font-mono text-[10px] uppercase tracking-wider font-semibold transition-colors cursor-pointer`}
      >
        <Library size={10} className="shrink-0" />
        <span>{hits.length} source{hits.length !== 1 ? 's' : ''}</span>
        <span className="ml-auto text-white/20">
          {isOpen ? <ChevronDown size={10} /> : <ChevronRight size={10} />}
        </span>
      </button>
      <AnimatePresence initial={false}>
        {isOpen && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.1 }}
            className="px-3 py-2 bg-black/10 border-t border-white/5"
          >
            {hits.map((hit) => (
              <div key={hit.number} className="mb-2 last:mb-0">
                <div className="flex items-center gap-2 mb-1">
                  <span className="font-mono text-[10px] text-secondary-txt/70 truncate">{hit.source}</span>
                  <span className={`ml-auto shrink-0 font-mono text-[9px] px-1 py-0.5 rounded ${accentClass}/20 ${accentClass} border ${borderClass}/30`}>
                    {hit.score.toFixed(2)}
                  </span>
                </div>
                <p className="font-mono text-[10px] text-secondary-txt/50 leading-relaxed whitespace-pre-wrap">
                  {hit.snippet.length > 200 ? hit.snippet.slice(0, 200) + '…' : hit.snippet}
                </p>
              </div>
            ))}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
};
