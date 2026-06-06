import { useEffect, useState } from "react";
import { tauriProvider } from "../../lib/tauri";

/**
 * Best-effort read of the workspace's default provider, used to run synthesis
 * through the CLI the user actually has authed. Falls back to `undefined` (the
 * engine then uses its default provider) if the preference can't be read.
 */
export function useLastUsedProvider(): string | undefined {
  const [provider, setProvider] = useState<string | undefined>(undefined);
  useEffect(() => {
    let active = true;
    void tauriProvider
      .getLastUsed()
      .then((r) => {
        if (active) setProvider(r.provider ?? undefined);
      })
      .catch(() => {
        // Already surfaced by the tauri call() wrapper; the engine's default
        // provider is a fine fallback for synthesis, so do not block the UI.
      });
    return () => {
      active = false;
    };
  }, []);
  return provider;
}
