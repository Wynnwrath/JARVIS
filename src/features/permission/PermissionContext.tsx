import { createContext, useContext, useEffect, useState, useRef, type ReactNode } from 'react';
import { onPermissionRequired, respondToPermission } from '@/services/permissionService';
import type { PermissionRequest, PermissionResponse } from '@/types/tauri';

interface PermissionContextValue {
  pendingRequests: PermissionRequest[];
  respond: (requestId: string, response: PermissionResponse) => Promise<void>;
  dismiss: (requestId: string) => Promise<void>;
}

const PermissionContext = createContext<PermissionContextValue | null>(null);

export function PermissionProvider({ children }: { children: ReactNode }) {
  const [pendingRequests, setPendingRequests] = useState<PermissionRequest[]>([]);
  const unlistenRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    onPermissionRequired((request) => {
      setPendingRequests((prev) => {
        if (prev.some((r) => r.request_id === request.request_id)) return prev;
        return [...prev, request];
      });
    })
      .then((unlisten) => {
        unlistenRef.current = unlisten;
      })
      .catch((err) => {
        console.error('[PermissionProvider] Failed to register permission-required listener:', err);
      });
    return () => {
      unlistenRef.current?.();
    };
  }, []);

  const respond = async (requestId: string, response: PermissionResponse) => {
    await respondToPermission(requestId, response);
    setPendingRequests((prev) => prev.filter((r) => r.request_id !== requestId));
  };

  const dismiss = async (requestId: string) => {
    await respondToPermission(requestId, { kind: 'deny', reason: 'User dismissed prompt' });
    setPendingRequests((prev) => prev.filter((r) => r.request_id !== requestId));
  };

  return (
    <PermissionContext.Provider value={{ pendingRequests, respond, dismiss }}>
      {children}
    </PermissionContext.Provider>
  );
}

export function usePermission() {
  const ctx = useContext(PermissionContext);
  if (!ctx) throw new Error('usePermission must be used within PermissionProvider');
  return ctx;
}
