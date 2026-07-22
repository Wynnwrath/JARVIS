/**
 * Configuration Service
 * 
 * Reads/writes application configuration via the Tauri backend,
 * which persists to `config.toml` on the host filesystem.
 */

import { invoke } from '@tauri-apps/api/core';
import { AppConfig } from '@/types/tauri';

// ─── Defaults (used as fallback if backend is unavailable) ──────────────────

export const DEFAULT_CONFIG: AppConfig = {
  provider: 'openai',
  api_key: '',
  chat_model: 'google/gemma-4-e4b',
  chat_base_url: 'http://127.0.0.1:1234/v1',
  silence_threshold_rms: 0.01,
  silence_duration_ms: 1000,
  transcription_model_path: 'parakeet-tdt-0.6b-v3-int8',
  system_prompt: 'You are JARVIS, a helpful AI assistant.',
  compaction_prompt: 'Summarize this context briefly, capturing key points.',
  compaction_threshold: 128000,
  database_name: 'jarvis.db',
  mcp_config_path: 'mcp.json',
  sandbox_dir: '.',
  sandbox_roots: [],
  read_extensions: ['txt', 'md', 'pdf', 'json', 'toml', 'rs', 'js', 'ts', 'tsx', 'html', 'css'],
  write_extensions: ['txt', 'md', 'json', 'toml', 'rs', 'js', 'ts', 'tsx', 'html', 'css'],
  rag_enabled: false,
  embedding_model: 'BGESmallENV15',
  rag_use_gpu: false,
  rag_exclusions: [],
};

// ─── Re-export the type for convenience ─────────────────────────────────────

export type { AppConfig };

// ─── Provider Defaults ──────────────────────────────────────────────────────

export const PROVIDER_BASE_URLS: Record<string, string> = {
  openai: 'https://api.openai.com/v1',
  gemini: 'https://generativelanguage.googleapis.com/v1beta',
  anthropic: 'https://api.anthropic.com/v1',
};

export const PROVIDER_MODEL_SUGGESTIONS: Record<string, string[]> = {
  openai: ['gpt-4o', 'gpt-4o-mini', 'gpt-4-turbo', 'gpt-3.5-turbo'],
  gemini: ['gemini-2.5-flash', 'gemini-2.5-pro', 'google/gemma-4-e4b'],
  anthropic: ['claude-sonnet-4-20250514', 'claude-3-5-sonnet-latest', 'claude-3-haiku-20240307'],
};

export const EMBEDDING_MODELS: { category: string; models: { name: string; dims: number }[] }[] = [
  { category: 'BAAI BGE', models: [
    { name: 'BGESmallENV15', dims: 384 }, { name: 'BGESmallENV15Q', dims: 384 },
    { name: 'BGEBaseENV15', dims: 768 }, { name: 'BGEBaseENV15Q', dims: 768 },
    { name: 'BGELargeENV15', dims: 1024 }, { name: 'BGELargeENV15Q', dims: 1024 },
    { name: 'BGESmallZHV15', dims: 512 }, { name: 'BGELargeZHV15', dims: 1024 },
    { name: 'BGEM3', dims: 1024 },
  ]},
  { category: 'Sentence Transformers', models: [
    { name: 'AllMiniLML6V2', dims: 384 }, { name: 'AllMiniLML6V2Q', dims: 384 },
    { name: 'AllMiniLML12V2', dims: 384 }, { name: 'AllMiniLML12V2Q', dims: 384 },
    { name: 'AllMpnetBaseV2', dims: 768 },
  ]},
  { category: 'Nomic', models: [
    { name: 'NomicEmbedTextV1', dims: 768 }, { name: 'NomicEmbedTextV15', dims: 768 },
    { name: 'NomicEmbedTextV15Q', dims: 768 },
  ]},
  { category: 'Multilingual E5', models: [
    { name: 'MultilingualE5Small', dims: 384 }, { name: 'MultilingualE5Base', dims: 768 },
    { name: 'MultilingualE5Large', dims: 1024 },
  ]},
  { category: 'GTE', models: [
    { name: 'GTEBaseENV15', dims: 768 }, { name: 'GTEBaseENV15Q', dims: 768 },
    { name: 'GTELargeENV15', dims: 1024 }, { name: 'GTELargeENV15Q', dims: 1024 },
  ]},
  { category: 'Snowflake Arctic', models: [
    { name: 'SnowflakeArcticEmbedXS', dims: 384 }, { name: 'SnowflakeArcticEmbedXSQ', dims: 384 },
    { name: 'SnowflakeArcticEmbedS', dims: 384 }, { name: 'SnowflakeArcticEmbedSQ', dims: 384 },
    { name: 'SnowflakeArcticEmbedM', dims: 768 }, { name: 'SnowflakeArcticEmbedMQ', dims: 768 },
    { name: 'SnowflakeArcticEmbedMLong', dims: 768 }, { name: 'SnowflakeArcticEmbedMLongQ', dims: 768 },
    { name: 'SnowflakeArcticEmbedL', dims: 1024 }, { name: 'SnowflakeArcticEmbedLQ', dims: 1024 },
  ]},
  { category: 'Jina', models: [
    { name: 'JinaEmbeddingsV2BaseCode', dims: 768 }, { name: 'JinaEmbeddingsV2BaseEN', dims: 768 },
  ]},
  { category: 'Other', models: [
    { name: 'MxbaiEmbedLargeV1', dims: 1024 }, { name: 'MxbaiEmbedLargeV1Q', dims: 1024 },
    { name: 'ModernBertEmbedLarge', dims: 1024 },
    { name: 'ParaphraseMLMiniLML12V2', dims: 384 }, { name: 'ParaphraseMLMiniLML12V2Q', dims: 384 },
    { name: 'ParaphraseMLMpnetBaseV2', dims: 768 },
    { name: 'EmbeddingGemma300M', dims: 768 }, { name: 'EmbeddingGemma300MQ', dims: 768 },
    { name: 'EmbeddingGemma300MQ4', dims: 768 },
    { name: 'ClipVitB32', dims: 512 },
  ]},
];

// ─── Service Methods ────────────────────────────────────────────────────────

/**
 * Fetches the current application configuration from the Rust backend.
 * Falls back to DEFAULT_CONFIG if the backend is unreachable.
 */
export const getConfig = async (): Promise<AppConfig> => {
  try {
    return await invoke<AppConfig>('get_config');
  } catch (err) {
    console.warn('[ConfigService] Backend get_config failed, using defaults:', err);
    return { ...DEFAULT_CONFIG };
  }
};

/**
 * Saves the application configuration via the Rust backend.
 * The backend writes the config to `config.toml` on disk.
 */
export const saveConfig = async (config: AppConfig): Promise<void> => {
  try {
    await invoke('update_config', { newConfig: config });
    console.log('[ConfigService] Config saved via backend.');
  } catch (err) {
    console.error('[ConfigService] Backend update_config failed:', err);
    throw err;
  }
};

/**
 * Resets configuration to defaults by saving defaults to the backend.
 */
export const resetConfig = async (): Promise<AppConfig> => {
  const defaults = { ...DEFAULT_CONFIG };
  try {
    await invoke('update_config', { newConfig: defaults });
    console.log('[ConfigService] Config reset to defaults via backend.');
  } catch (err) {
    console.warn('[ConfigService] Backend reset failed, returning local defaults:', err);
  }
  return defaults;
};
