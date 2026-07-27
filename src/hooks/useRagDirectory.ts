import { useState, useEffect, useCallback } from 'react';
import {
  listRagDirectory, readRagDocument, RagDirEntry
} from '@/services/ragService';

export const useRagDirectory = (rootPath: string | null) => {
  const [currentPath, setCurrentPath] = useState<string>(rootPath ?? "");
  const [files, setFiles] = useState<RagDirEntry[]>([]);
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [selectedFileContent, setSelectedFileContent] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadDirectory = useCallback(async (path: string) => {
    setLoading(true);
    setError(null);
    try {
      const parsedFiles = await listRagDirectory(path);
      setFiles(parsedFiles);
      setCurrentPath(path);
      setSelectedFile(null);
      setSelectedFileContent(null);
    } catch (err: any) {
      setError(err?.message || "Failed to load directory");
      console.error("[useRagDirectory] Error listing directory:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  const selectFile = useCallback(async (filePath: string) => {
    setLoading(true);
    setError(null);
    try {
      const content = await readRagDocument(filePath);
      setSelectedFile(filePath);
      setSelectedFileContent(content);
    } catch (err: any) {
      setError(err?.message || `Failed to read document: ${filePath}`);
      console.error("[useRagDirectory] Error reading file:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  const goBack = useCallback(() => {
    if (!rootPath || !currentPath || currentPath === rootPath) return;
    const normalized = currentPath.replace(/[/\\]$/, "");
    const parts = normalized.split(/[/\\]/);
    parts.pop();
    let parentPath = parts.join("/");
    if (!parentPath.startsWith(rootPath)) {
      parentPath = rootPath;
    }
    loadDirectory(parentPath);
  }, [currentPath, rootPath, loadDirectory]);

  useEffect(() => {
    if (rootPath) {
      loadDirectory(rootPath);
    } else {
      setFiles([]);
      setSelectedFile(null);
      setSelectedFileContent(null);
      setCurrentPath("");
    }
  }, [rootPath, loadDirectory]);

  return {
    currentPath,
    files,
    selectedFile,
    selectedFileContent,
    loading,
    error,
    setSelectedFile,
    setSelectedFileContent,
    loadDirectory,
    selectFile,
    goBack
  };
};
