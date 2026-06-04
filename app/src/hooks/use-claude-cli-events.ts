import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import type { HoustonEvent } from "@houston-ai/core";
import { subscribeHoustonEvents } from "../lib/events";
import { tauriProvider } from "../lib/tauri";
import { useUIStore } from "../stores/ui";
import { useClaudeInstallErrorText } from "./use-claude-install";
import { logger } from "../lib/logger";

/**
 * Subscriber for the `claude` WS topic — closes the loop on #231.
 *
 * Engine emits three lifecycle events from `houston-claude-installer`:
 *
 * - `ClaudeCliInstalling { progress_pct }` — recurring during the
 *   ~120 MB download. We intentionally do NOT toast each tick (10%
 *   increments × 5 = 5 toasts a user would dismiss); the install
 *   progress UI itself is a separate surface tracked elsewhere.
 * - `ClaudeCliReady` — install finished (or already at the pinned
 *   version). Re-runs the provider status check so the Anthropic chip
 *   and "claudeAvailable" gate flip without a launch.
 * - `ClaudeCliFailed { error }` — fatal install error. `error` is the
 *   typed `ClaudeInstallError` taxonomy (kind + optional status / platform
 *   / detail). We localize it to a user-facing string via
 *   `useClaudeInstallErrorText` and surface it through `addToast` as an
 *   error variant — the toast container renders plain text with the
 *   error icon and dismiss control (`ui/core/src/components/toast-container.tsx`).
 *
 * Mounted once in `App.tsx` next to `useAgentInvalidation`. Idempotent.
 */
export function useClaudeCliEvents() {
  const { t } = useTranslation("shell");
  const addToast = useUIStore((s) => s.addToast);
  const setClaudeAvailable = useUIStore((s) => s.setClaudeAvailable);
  const installErrorText = useClaudeInstallErrorText();

  useEffect(() => {
    const unlisten = subscribeHoustonEvents((p: HoustonEvent) => {
      switch (p.type) {
        case "ClaudeCliInstalling":
          // No-op — progress UI lives in its own surface. Log only so
          // we have a breadcrumb if the install hangs.
          logger.debug(
            `[claude-cli] installing: ${p.data.progress_pct}%`,
          );
          break;
        case "ClaudeCliReady":
          logger.info("[claude-cli] ready");
          // Re-run the provider status check — the install just landed
          // and the user's claudeAvailable gate (used by chat / agent
          // creation) should flip without requiring a relaunch. The
          // check is cheap (one Tauri command) and the same path
          // `useHoustonInit` uses on startup, so behavior stays
          // consistent.
          tauriProvider
            .checkStatus("anthropic")
            .then((status) => {
              setClaudeAvailable(
                status.cli_installed && status.authenticated,
              );
            })
            .catch((e) => {
              // The status check failing isn't user-actionable here —
              // worst case the user sees the chip flip on next launch.
              // Log so support has a breadcrumb if they ask why.
              logger.warn(
                `[claude-cli] post-install status check failed: ${e}`,
              );
            });
          break;
        case "ClaudeCliFailed": {
          // Localize the typed install error to a user-facing string and
          // surface it per CLAUDE.md §"No silent failures" — the toast IS
          // the user-facing error.
          const description = installErrorText(p.data.error);
          logger.error(`[claude-cli] failed: ${p.data.error.kind}`);
          addToast({
            title: t("claudeCli.installFailedTitle"),
            description,
            variant: "error",
          });
          break;
        }
      }
    });

    return () => {
      unlisten();
    };
  }, [addToast, setClaudeAvailable, t, installErrorText]);
}
