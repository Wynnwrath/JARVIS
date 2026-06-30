import { invoke } from '@tauri-apps/api/core';

export async function addSandboxRoot(root: string): Promise<void> {
  return invoke('add_sandbox_root', { root });
}

export async function removeSandboxRoot(root: string): Promise<void> {
  return invoke('remove_sandbox_root', { root });
}

export async function listSandboxRoots(): Promise<string[]> {
  return invoke('list_sandbox_roots');
}
