import { motion, AnimatePresence } from 'framer-motion';
import { ShieldAlert } from 'lucide-react';
import { usePermission } from '../PermissionContext';

const KNOWN_PREFIXES = [
  'Wants to write file at ',
  'Wants to read file at ',
  'Wants to list directory ',
  'Wants to search files in ',
  'Wants to grep files in ',
];

const DIR_PREFIXES = new Set([
  'Wants to list directory ',
  'Wants to search files in ',
  'Wants to grep files in ',
]);

function extractPathFromDescription(description: string): { path: string; isDirectoryContext: boolean } | null {
  for (const prefix of KNOWN_PREFIXES) {
    if (description.startsWith(prefix)) {
      let rest = description.slice(prefix.length).trimStart();
      rest = rest.replace(/^\[/, '').replace(/\]$/, '').trim();
      if (rest.length > 0) return { path: rest, isDirectoryContext: DIR_PREFIXES.has(prefix) };
    }
  }
  return null;
}

export function PermissionPromptOverlay() {
  const { pendingRequests, respond, dismiss } = usePermission();

  if (pendingRequests.length === 0) return null;

  return (
    <div className="fixed top-20 right-4 z-50 flex flex-col gap-3 max-w-sm">
      <AnimatePresence>
        {pendingRequests.map((req) => (
          <motion.div
            key={req.request_id}
            initial={{ opacity: 0, x: 80, scale: 0.95 }}
            animate={{ opacity: 1, x: 0, scale: 1 }}
            exit={{ opacity: 0, x: 80, scale: 0.95 }}
            transition={{ type: 'spring', damping: 25, stiffness: 300 }}
            className="border border-theme-border bg-theme-surface-1/95 backdrop-blur-xl rounded-lg p-4 shadow-[0_8px_32px_rgba(0,0,0,0.4)]"
          >
            <div className="flex items-start gap-3 mb-3">
              <div className="shrink-0 mt-0.5">
                <ShieldAlert size={18} className="text-theme-accent" />
              </div>
              <div className="min-w-0">
                <h3 className="text-theme-accent font-mono text-[10px] uppercase tracking-[0.2em] font-bold">
                  Permission Required
                </h3>
                <p className="text-primary-txt font-mono text-xs mt-1.5 font-semibold">
                  {req.tool_name}
                </p>
                <p className="text-secondary-txt text-[11px] mt-1 leading-relaxed">
                  {req.description}
                </p>
              </div>
            </div>

            {(() => {
              const extracted = extractPathFromDescription(req.description);
              const dir = extracted
                ? extracted.isDirectoryContext
                  ? extracted.path
                  : extracted.path.slice(0, extracted.path.lastIndexOf('/')) || '/'
                : null;
              return (
                <div className="grid grid-cols-2 gap-2 mt-3">
                  <button
                    onClick={() => respond(req.request_id, { kind: 'allow' })}
                    className="px-3 py-1.5 bg-green-600/80 hover:bg-green-500/80 text-white text-[11px] font-mono font-semibold rounded transition-colors cursor-pointer"
                  >
                    Allow Once
                  </button>
                  <button
                    onClick={() => dismiss(req.request_id)}
                    className="px-3 py-1.5 bg-red-600/80 hover:bg-red-500/80 text-white text-[11px] font-mono font-semibold rounded transition-colors cursor-pointer"
                  >
                    Deny
                  </button>
                  {dir ? (
                    <>
                      <button
                        onClick={() => respond(req.request_id, { kind: 'allow_always', path: dir })}
                        className="col-span-2 px-3 py-1.5 bg-green-700/70 hover:bg-green-600/70 text-white text-[11px] font-mono font-semibold rounded transition-colors cursor-pointer truncate"
                        title={dir}
                      >
                        Always allow in this directory
                      </button>
                      <button
                        onClick={() => respond(req.request_id, { kind: 'deny_always', path: dir })}
                        className="col-span-2 px-3 py-1.5 bg-red-700/70 hover:bg-red-600/70 text-white text-[11px] font-mono font-semibold rounded transition-colors cursor-pointer truncate"
                        title={dir}
                      >
                        Always deny in this directory
                      </button>
                    </>
                  ) : null}
                </div>
              );
            })()}
          </motion.div>
        ))}
      </AnimatePresence>
    </div>
  );
}
