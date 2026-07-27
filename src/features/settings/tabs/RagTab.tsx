import { Info } from 'lucide-react';
import { AppConfig, EMBEDDING_MODELS } from '@/services/configService';
import { SectionHeader, FieldGroup } from '../components/FieldGroup';

interface TabProps {
  config: AppConfig;
  updateConfig: <K extends keyof AppConfig>(key: K, value: AppConfig[K]) => void;
  accent: string;
}

const ToggleSwitch = ({ enabled, onChange, label }: { enabled: boolean; onChange: (v: boolean) => void; label: string }) => (
  <div className="flex items-center justify-between">
    <span className="text-xs font-mono text-secondary-txt uppercase tracking-wider">{label}</span>
    <button
      onClick={() => onChange(!enabled)}
      className={`relative w-10 h-5 rounded-full transition-colors duration-300 border cursor-pointer
        ${enabled ? 'bg-offline-core/20 border-offline-core/50' : 'bg-white/5 border-white/10'}`}
    >
      <div
        className={`absolute top-0.5 w-4 h-4 rounded-full transition-all duration-300
          ${enabled ? 'left-5 bg-offline-core shadow-[0_0_6px_var(--color-offline-core)]' : 'left-0.5 bg-secondary-txt/40'}`}
      />
    </button>
  </div>
);

export const RagTab = ({ config, updateConfig }: TabProps) => {
  return (
    <div className="space-y-7">
      <SectionHeader title="RAG_Configuration" subtitle="Local semantic search and embedding pipeline settings" />

      <FieldGroup label="Enable RAG" description="Enable local Retrieval-Augmented Generation for offline document search.">
        <ToggleSwitch
          enabled={config.rag_enabled}
          onChange={(v) => updateConfig('rag_enabled', v)}
          label="RAG Engine"
        />
      </FieldGroup>

      <FieldGroup label="Embedding Model" description="Vector model used to embed document chunks for semantic search.">
        <div className="relative">
          <select
            value={config.embedding_model}
            onChange={(e) => updateConfig('embedding_model', e.target.value)}
            className="w-full bg-white/[0.02] hover:bg-white/[0.04] border border-white/10 rounded-lg px-4 py-3 text-sm font-mono text-primary-txt focus:outline-none focus:border-[var(--theme-accent)]/50 focus:ring-1 focus:ring-[var(--theme-accent)]/30 transition-all duration-300 appearance-none cursor-pointer"
          >
            {EMBEDDING_MODELS.map((group) => (
              <optgroup key={group.category} label={group.category} className="bg-[#121214] text-primary-txt">
                {group.models.map((m) => (
                  <option
                    key={m.name}
                    value={m.name}
                    className="bg-[#121214] text-primary-txt"
                  >
                    {m.name} ({m.dims}d)
                  </option>
                ))}
              </optgroup>
            ))}
          </select>
        </div>
        <div className="flex items-start gap-2 mt-2 text-[10px] font-mono text-warning-orange/70">
          <Info size={10} className="mt-0.5 shrink-0" />
          <span>Switching embedding models requires a full re-index of all documents.</span>
        </div>
      </FieldGroup>

      <FieldGroup label="GPU Acceleration" description="Requires a GPU-enabled ONNX Runtime installed on your system. Discovered automatically via system library paths. Falls back to CPU if unavailable.">
        <ToggleSwitch
          enabled={config.rag_use_gpu}
          onChange={(v) => updateConfig('rag_use_gpu', v)}
          label="GPU Inference"
        />
      </FieldGroup>
    </div>
  );
};
