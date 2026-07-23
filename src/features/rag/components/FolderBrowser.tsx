import { Folder, ChevronLeft, Plus, X } from 'lucide-react';

interface FolderBrowserProps {
  folders: string[];
  selectedFolder: string | null;
  onSelect: (folder: string) => void;
  onBack: () => void;
  ragDirs: string[];
  activeRagDir: string | null;
  onAddDir: () => void;
  onSelectRagDir: (path: string) => void;
  onRemoveRagDir: (path: string) => void;
}

const dirDisplayName = (path: string) =>
  path.replace(/[/\\]$/, "").split(/[/\\]/).pop() || path;

export const FolderBrowser = ({
  folders, selectedFolder, onSelect, onBack,
  ragDirs, activeRagDir, onAddDir, onSelectRagDir, onRemoveRagDir
}: FolderBrowserProps) => {
  return (
    <div className="bg-offline-surface-dark border border-offline-border rounded-xl p-4 h-full flex flex-col">
      <div className="flex items-center gap-2 mb-3">
        {selectedFolder && (
          <button onClick={onBack} className="p-1 hover:bg-white/5 rounded cursor-pointer">
            <ChevronLeft size={14} className="text-offline-core" />
          </button>
        )}
        <span className="text-[10px] font-mono text-offline-core uppercase tracking-widest font-bold flex-1 truncate">
          {selectedFolder ? selectedFolder : 'Workspace Folders'}
        </span>
        <button
          onClick={onAddDir}
          title="Add directory to index"
          className="p-1 rounded border border-offline-core/30 text-offline-core/70 hover:text-offline-core hover:border-offline-core/60 hover:bg-offline-core/10 transition-colors cursor-pointer"
        >
          <Plus size={12} />
        </button>
      </div>
      <div className="flex-1 overflow-y-auto custom-scrollbar space-y-3">
        <div className="space-y-1">
          <span className="block text-[8px] font-mono text-tertiary-txt/60 uppercase tracking-widest px-3">
            Workspace
          </span>
          {folders.length === 0 && (
            <span className="block text-[9px] font-mono text-tertiary-txt/40 px-3 py-1">
              No workspace set
            </span>
          )}
          {folders.map((folder) => (
            <button
              key={folder}
              onClick={() => onSelect(folder)}
              className="w-full text-left flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-offline-core/5 text-secondary-txt hover:text-offline-core transition-colors font-mono text-[11px] cursor-pointer"
            >
              <Folder size={14} className="shrink-0" />
              <span className="truncate">{dirDisplayName(folder)}</span>
            </button>
          ))}
        </div>

        <div className="space-y-1">
          <span className="block text-[8px] font-mono text-tertiary-txt/60 uppercase tracking-widest px-3">
            Indexed Dirs
          </span>
          {ragDirs.length === 0 && (
            <span className="block text-[9px] font-mono text-tertiary-txt/40 px-3 py-1">
              None added — use +
            </span>
          )}
          {ragDirs.map((dir) => {
            const active = activeRagDir === dir;
            return (
              <div
                key={dir}
                onClick={() => onSelectRagDir(dir)}
                title={dir}
                className={`group w-full text-left flex items-center gap-3 px-3 py-2 rounded-lg transition-colors font-mono text-[11px] cursor-pointer
                  ${active
                    ? 'bg-offline-core/10 text-offline-core border border-offline-core/30'
                    : 'text-secondary-txt hover:text-offline-core hover:bg-offline-core/5 border border-transparent'}`}
              >
                <Folder size={14} className="shrink-0" />
                <span className="truncate flex-1">{dirDisplayName(dir)}</span>
                <button
                  onClick={(e) => { e.stopPropagation(); onRemoveRagDir(dir); }}
                  title="Remove and de-index"
                  className="opacity-0 group-hover:opacity-100 p-0.5 rounded text-tertiary-txt/60 hover:text-red-400 hover:bg-red-400/10 transition-all cursor-pointer shrink-0"
                >
                  <X size={11} />
                </button>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
};
