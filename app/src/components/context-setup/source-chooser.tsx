import { useTranslation } from "react-i18next";
import { Button } from "@houston-ai/core";
import {
  Brain,
  FileJson,
  Folder,
  MessageSquareText,
  NotebookText,
  X,
} from "lucide-react";
import type { ImportSource, ImportSourceKind } from "@houston-ai/engine-client";
import { osPickDirectory, osPickFile } from "../../lib/os-bridge";
import { SOURCE_KINDS, type SourceKindConfig } from "./helpers";

const ICONS: Record<ImportSourceKind, typeof Folder> = {
  localFolder: Folder,
  claudeHome: Brain,
  obsidianVault: NotebookText,
  chatGptExport: MessageSquareText,
  claudeAiExport: FileJson,
};

function basename(path: string): string {
  const parts = path.replace(/\/+$/, "").split("/");
  return parts[parts.length - 1] || path;
}

interface Props {
  sources: ImportSource[];
  onAddSource: (src: ImportSource) => void;
  onRemoveSource: (index: number) => void;
  onContinue: () => void;
  onSkip: () => void;
}

export function SourceChooser({
  sources,
  onAddSource,
  onRemoveSource,
  onContinue,
  onSkip,
}: Props) {
  const { t } = useTranslation("contextSetup");

  const pickFor = async (cfg: SourceKindConfig) => {
    let path: string | null = null;
    if (cfg.picker === "default") path = cfg.defaultPath ?? null;
    else if (cfg.picker === "folder") path = await osPickDirectory();
    else path = await osPickFile();
    if (path) onAddSource({ kind: cfg.kind, path });
  };

  return (
    <div className="flex flex-col gap-5">
      <div className="flex flex-col gap-1">
        <h3 className="text-sm font-semibold text-foreground">
          {t("sources.heading")}
        </h3>
        <p className="text-xs text-muted-foreground max-w-prose">
          {t("sources.description")}
        </p>
      </div>

      <div className="grid gap-2">
        {SOURCE_KINDS.map((cfg) => {
          const Icon = ICONS[cfg.kind];
          const actionKey =
            cfg.picker === "default"
              ? "sources.useThis"
              : cfg.picker === "folder"
                ? "sources.chooseFolder"
                : "sources.chooseFile";
          return (
            <button
              key={cfg.kind}
              type="button"
              onClick={() => void pickFor(cfg)}
              className="flex items-center gap-3 rounded-xl bg-secondary p-3 text-left transition-colors duration-200 hover:bg-accent w-full"
            >
              <Icon className="size-5 shrink-0 text-muted-foreground" />
              <span className="flex-1 min-w-0">
                <span className="block text-sm font-medium text-foreground">
                  {t(`sources.kinds.${cfg.kind}.label`)}
                </span>
                <span className="block text-xs text-muted-foreground truncate">
                  {t(`sources.kinds.${cfg.kind}.description`)}
                </span>
              </span>
              <span className="text-xs font-medium text-foreground/70 shrink-0">
                {t(actionKey)}
              </span>
            </button>
          );
        })}
      </div>

      {sources.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {sources.map((src, i) => (
            <span
              key={`${src.kind}:${src.path}`}
              className="inline-flex items-center gap-1.5 rounded-full bg-accent px-3 py-1 text-xs text-foreground"
            >
              {basename(src.path)}
              <button
                type="button"
                aria-label={t("sources.remove")}
                onClick={() => onRemoveSource(i)}
                className="text-muted-foreground hover:text-foreground"
              >
                <X className="size-3" />
              </button>
            </span>
          ))}
        </div>
      )}

      <div className="flex items-center justify-between gap-3 pt-1">
        <button
          type="button"
          onClick={onSkip}
          className="text-xs text-muted-foreground hover:text-foreground"
        >
          {t("sources.skipImport")}
        </button>
        <Button onClick={onContinue} disabled={sources.length === 0}>
          {t("sources.continue")}
        </Button>
      </div>
      {sources.length === 0 && (
        <p className="text-[11px] text-muted-foreground -mt-3">
          {t("sources.empty")}
        </p>
      )}
    </div>
  );
}
