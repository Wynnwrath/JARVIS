import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import type { PermissionRequest, PermissionPreference, PermissionResponse } from '@/types/tauri';

export async function respondToPermission(
  requestId: string,
  response: PermissionResponse
): Promise<void> {
  return invoke('respond_to_permission', { requestId, response });
}

export async function getPermissionPreferences(): Promise<PermissionPreference[]> {
  return invoke('get_permission_preferences');
}

export async function setPermissionPreference(
  toolName: string,
  decision: 'allow' | 'deny',
  pathPattern?: string | null
): Promise<void> {
  return invoke('set_permission_preference', { toolName, decision, pathPattern: pathPattern ?? null });
}

export async function deletePermissionPreference(
  toolName: string,
  pathPattern?: string | null
): Promise<void> {
  return invoke('delete_permission_preference', { toolName, pathPattern: pathPattern ?? null });
}

export async function onPermissionRequired(
  callback: (request: PermissionRequest) => void
): Promise<UnlistenFn> {
  return listen<PermissionRequest>('permission-required', (e) => callback(e.payload));
}
