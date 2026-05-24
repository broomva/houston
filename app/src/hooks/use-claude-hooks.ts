/**
 * TanStack Query hooks for `/v1/claude-hooks/*` — Phase 7 of RFC #248
 * (`advanced.claude_hooks`).
 *
 * The engine is the source of truth for install state — we never write
 * `settings.json` from the frontend. Install / uninstall mutations
 * return the new `ClaudeHookStatus` so the cache is refreshed inline
 * without an extra GET.
 */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { ClaudeHookStatus } from "@houston-ai/engine-client";
import { tauriClaudeHooks } from "../lib/tauri";

const QUERY_KEY = ["claude-hooks", "status"] as const;
const STALE_MS = 10_000;

export function useClaudeHookStatus(enabled = true) {
  return useQuery<ClaudeHookStatus>({
    queryKey: QUERY_KEY,
    queryFn: () => tauriClaudeHooks.status(),
    enabled,
    staleTime: STALE_MS,
    refetchOnWindowFocus: false,
  });
}

export function useInstallClaudeHooks() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => tauriClaudeHooks.install(),
    onSuccess: (status) => {
      qc.setQueryData(QUERY_KEY, status);
    },
  });
}

export function useUninstallClaudeHooks() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => tauriClaudeHooks.uninstall(),
    onSuccess: (status) => {
      qc.setQueryData(QUERY_KEY, status);
    },
  });
}
