import { useState, useEffect, useMemo } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { getRagTelemetry, clearRagDatabase, startRagIndexing, removeRagDir, RagTelemetry } from '@/services/ragService';
import { useDocument } from '@/hooks/useDocument';
import { useRagDirectory } from '@/hooks/useRagDirectory';
import { getConfig, saveConfig } from '@/services/configService';
import { RAGHeader } from '@/features/rag/components/RAGHeader';
import { FolderBrowser } from '@/features/rag/components/FolderBrowser';
import { DocumentExplorer } from '@/features/rag/components/DocumentExplorer';
import { FilePreview } from '@/features/rag/components/FilePreview';
import { PipelineController } from '@/features/rag/components/PipelineController';
import { PipelineConsole } from '@/features/rag/components/PipelineConsole';
import { SearchPanel } from '@/features/rag/components/SearchPanel';
import { RAGFooter } from '@/features/rag/components/RAGFooter';

export const OfflineRAGPage = () => {
  const [vaultPath, setVaultPath] = useState('');
  const [sandboxDir, setSandboxDir] = useState('');
  const [isSyncing, setIsSyncing] = useState(false);
  const [syncProgress, setSyncProgress] = useState(0);
  const [logs, setLogs] = useState<string[]>([
    '[INIT] Node.AirGapped RAG system initialized.',
    '[STATUS] Active vector database SQLite-VSS connected.',
    '[READY] Browse your workspace folders and select documents to preview.'
  ]);

  const [stats, setStats] = useState<RagTelemetry | null>(null);
  const {
    currentPath, files: documentFiles, selectedFile, selectedFileContent,
    loading: docLoading, error: docError,
    selectFile, loadDirectory, goBack
  } = useDocument(vaultPath);

  const [ragDirs, setRagDirs] = useState<string[]>([]);
  const [activeRagDir, setActiveRagDir] = useState<string | null>(null);
  const ragDir = useRagDirectory(activeRagDir);

  useEffect(() => {
    const initConfig = async () => {
      try {
        const config = await getConfig();
        if (config.sandbox_dir) {
          setSandboxDir(config.sandbox_dir);
        }
        setRagDirs(config.rag_dirs ?? []);
      } catch (err) {
        console.error('[OfflineRAGPage] Failed to fetch sandbox config:', err);
      }
    };
    initConfig();
  }, []);

  const refreshRagData = async () => {
    try {
      const tel = await getRagTelemetry();
      setStats(tel);
    } catch (err) {
      console.error('[OfflineRAGPage] Error loading data:', err);
    }
  };

  useEffect(() => {
    refreshRagData();
  }, [vaultPath]);

  const handleSelectPath = async () => {
    try {
      const selected = await open({ directory: true, multiple: false, title: 'Target Local Document Folder' });
      if (selected && typeof selected === 'string') {
        const config = await getConfig();
        await saveConfig({ ...config, sandbox_dir: selected });
        setVaultPath(selected);
        setLogs(prev => [...prev, `[WORKSPACE] Workspace set to: ${selected}`, '[DISCOVERY] Loading folder entries...']);
      }
    } catch (err) {
      console.error('[OfflineRAGPage] Failed to select path:', err);
    }
  };

  const handleAddRagDir = async () => {
    try {
      const selected = await open({ directory: true, multiple: false, title: 'Add Directory to Index' });
      if (selected && typeof selected === 'string') {
        const normalized = selected.replace(/[/\\]$/, "");
        if (ragDirs.includes(normalized)) {
          setLogs(prev => [...prev, `[DISCOVERY] Already indexed: ${normalized}`]);
          return;
        }
        const config = await getConfig();
        await saveConfig({ ...config, rag_dirs: [...(config.rag_dirs ?? []), normalized] });
        setRagDirs(prev => [...prev, normalized]);
        setLogs(prev => [...prev, `[DISCOVERY] Indexed dir added: ${normalized}`]);
      }
    } catch (err) {
      console.error('[OfflineRAGPage] Failed to add indexed dir:', err);
      setLogs(prev => [...prev, `[ERROR] Failed to add indexed dir: ${err}`]);
    }
  };

  const handleRemoveRagDir = async (dir: string) => {
    try {
      await removeRagDir(dir);
      const config = await getConfig();
      setRagDirs(config.rag_dirs ?? []);
      if (activeRagDir && (activeRagDir === dir || activeRagDir.startsWith(dir))) {
        setActiveRagDir(null);
      }
      setLogs(prev => [...prev, `[CLEANUP] Indexed dir removed and de-indexed: ${dir}`]);
    } catch (err) {
      console.error('[OfflineRAGPage] Failed to remove indexed dir:', err);
      setLogs(prev => [...prev, `[ERROR] Failed to remove indexed dir: ${err}`]);
    }
  };

  const handleStartIndexing = async () => {
    setIsSyncing(true);
    setSyncProgress(0);
    setLogs(prev => [...prev, '[PIPELINE] Starting document indexing pipeline...']);
    try {
      const interval = setInterval(() => setSyncProgress(p => Math.min(p + 10, 90)), 800);
      await startRagIndexing(vaultPath || sandboxDir, (payload) => {
        if (payload.message) setLogs(prev => [...prev, `[${payload.level}] ${payload.message}`]);
        setSyncProgress(payload.progress);
      });
      clearInterval(interval);
      setSyncProgress(100);
      setLogs(prev => [...prev, '[PIPELINE] Indexing complete.']);
      await refreshRagData();
      setTimeout(() => { setIsSyncing(false); setSyncProgress(0); }, 800);
    } catch (err) {
      console.error('[OfflineRAGPage] Indexing failed:', err);
      setLogs(prev => [...prev, `[ERROR] Indexing failed: ${err}`]);
      setIsSyncing(false);
    }
  };

  const handleClearDatabase = async () => {
    if (!window.confirm('WARNING: Proceeding will wipe all local vector database entries and indexed embeddings. This action cannot be undone. Continue?')) {
      return;
    }
    try {
      await clearRagDatabase();
      setLogs(prev => [...prev, '[PIPELINE] Vector database cleared.']);
      setStats(prev => prev ? { ...prev, totalChunks: 0, totalNotes: 0, indexedNotes: 0, dbSize: '0 KB' } : null);
    } catch (err) {
      console.error('[OfflineRAGPage] Wipe failed:', err);
      setLogs(prev => [...prev, `[ERROR] Wipe failed: ${err}`]);
    }
  };

  const folders = useMemo(() => documentFiles.filter(f => f.is_dir).map(f => f.name), [documentFiles]);
  const docs = useMemo(() => documentFiles.filter(f => !f.is_dir).map(f => ({ name: f.name, path: f.path })), [documentFiles]);
  const ragDocs = useMemo(() => ragDir.files.filter(f => !f.is_dir).map(f => ({ name: f.name, path: f.path })), [ragDir.files]);

  const activeRagDirName = activeRagDir
    ? activeRagDir.replace(/[/\\]$/, "").split(/[/\\]/).pop() || activeRagDir
    : null;
  const browserSelected = activeRagDirName ?? (currentPath ? currentPath.split(/[/\\]/).pop() || null : null);

  const handleBrowserBack = () => {
    if (activeRagDir) {
      if (ragDir.currentPath === activeRagDir) {
        setActiveRagDir(null);
      } else {
        ragDir.goBack();
      }
    } else {
      goBack();
    }
  };

  return (
    <div className="h-full flex flex-col p-6 bg-offline-bg">
      <RAGHeader vaultPath={vaultPath || sandboxDir} stats={stats} onSelectPath={handleSelectPath} />

      <div className="flex-1 grid grid-cols-[240px_1fr_280px] grid-rows-[1fr] gap-4 min-h-0">
        {/* Column 1: Folder Browser */}
        <FolderBrowser
          folders={folders}
          selectedFolder={browserSelected}
          onSelect={(folder) => { setActiveRagDir(null); loadDirectory(folder); }}
          onBack={handleBrowserBack}
          ragDirs={ragDirs}
          activeRagDir={activeRagDir}
          onAddDir={handleAddRagDir}
          onSelectRagDir={(path) => setActiveRagDir(path)}
          onRemoveRagDir={handleRemoveRagDir}
        />

        {/* Column 2: Document List / File Preview */}
        {activeRagDir ? (
          ragDir.selectedFile && !ragDir.loading ? (
            <FilePreview
              content={ragDir.error ? `**Error reading file:**\n\n${ragDir.error}` : (ragDir.selectedFileContent || '_(empty document)_')}
              fileName={ragDir.selectedFile.split(/[/\\]/).pop() || ragDir.selectedFile}
              onClose={() => ragDir.setSelectedFile('')}
            />
          ) : (
            <DocumentExplorer
              documents={ragDocs}
              onSelect={(path) => ragDir.selectFile(path)}
              selectedDocument={ragDir.selectedFile}
            />
          )
        ) : selectedFile && !docLoading ? (
          <FilePreview
            content={docError ? `**Error reading file:**\n\n${docError}` : (selectedFileContent || '_(empty document)_')}
            fileName={selectedFile.split('/').pop() || selectedFile}
            onClose={() => selectFile('')}
          />
        ) : (
          <DocumentExplorer
            documents={docs}
            onSelect={(path) => selectFile(path)}
            selectedDocument={selectedFile}
          />
        )}

        {/* Column 3: Pipeline Controller + Search + Console */}
        <div className="flex flex-col gap-4 min-h-0">
          <PipelineController
            isSyncing={isSyncing}
            syncProgress={syncProgress}
            onStartIndexing={handleStartIndexing}
            onClearDatabase={handleClearDatabase}
            onRefresh={refreshRagData}
          />
          <SearchPanel />
          <div className="flex-1 min-h-0">
            <PipelineConsole logs={logs} />
          </div>
        </div>
      </div>

      <RAGFooter />
    </div>
  );
};
