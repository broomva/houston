/**
 * `<ClaudeHooksPanel />` — settings sub-panel for `advanced.claude_hooks`.
 * Phase 7 of RFC #248.
 *
 * Renders only when the flag is on (the parent passes it through
 * `<FeatureGate>` from `../FeatureGate.tsx`). Shows install status,
 * install / uninstall buttons, and the path the hooks log to so the
 * user can `tail -f` it in their terminal.
 */
import { useTranslation } from "react-i18next";
import { Spinner } from "@houston-ai/core";
import {
  useClaudeHookStatus,
  useInstallClaudeHooks,
  useUninstallClaudeHooks,
} from "../../hooks/use-claude-hooks";

export function ClaudeHooksPanel() {
  const { t } = useTranslation("claudeHooks");
  const status = useClaudeHookStatus();
  const install = useInstallClaudeHooks();
  const uninstall = useUninstallClaudeHooks();

  const installed = (status.data?.houstonHookCount ?? 0) > 0;
  const busy = install.isPending || uninstall.isPending || status.isFetching;

  return (
    <section className="mt-6 rounded-xl border border-border bg-card px-4 py-4">
      <header className="mb-3">
        <h3 className="text-sm font-semibold">{t("title")}</h3>
        <p className="mt-0.5 text-xs text-muted-foreground">{t("description")}</p>
      </header>

      <dl className="grid grid-cols-[max-content_1fr] gap-x-3 gap-y-1 text-xs">
        <dt className="text-muted-foreground">{t("fields.status")}</dt>
        <dd className="font-mono">
          {status.isPending ? (
            <Spinner className="size-3" />
          ) : installed ? (
            <span className="text-emerald-600 dark:text-emerald-400">
              {t("status.installed", { count: status.data?.houstonHookCount ?? 0 })}
            </span>
          ) : (
            <span className="text-muted-foreground">{t("status.notInstalled")}</span>
          )}
        </dd>

        <dt className="text-muted-foreground">{t("fields.settingsPath")}</dt>
        <dd className="font-mono break-all">
          {status.data?.settingsPath ?? "—"}
        </dd>

        <dt className="text-muted-foreground">{t("fields.eventsLog")}</dt>
        <dd className="font-mono break-all">
          {status.data?.eventsLogPath ?? "—"}
        </dd>
      </dl>

      <div className="mt-4 flex items-center gap-2">
        {installed ? (
          <button
            type="button"
            onClick={() => uninstall.mutate()}
            disabled={busy}
            className="px-3 py-1.5 text-sm rounded-md border border-border text-foreground hover:bg-accent transition-colors disabled:opacity-50"
          >
            {uninstall.isPending ? t("actions.uninstalling") : t("actions.uninstall")}
          </button>
        ) : (
          <button
            type="button"
            onClick={() => install.mutate()}
            disabled={busy}
            className="px-3 py-1.5 text-sm rounded-md bg-primary text-primary-foreground hover:bg-primary/90 transition-colors disabled:opacity-50"
          >
            {install.isPending ? t("actions.installing") : t("actions.install")}
          </button>
        )}
        {status.data && status.data.totalHookCount > status.data.houstonHookCount ? (
          <span className="text-xs text-muted-foreground">
            {t("status.coexisting", {
              count: status.data.totalHookCount - status.data.houstonHookCount,
            })}
          </span>
        ) : null}
      </div>

      <p className="mt-3 text-xs text-muted-foreground">{t("tailHint")}</p>
    </section>
  );
}
