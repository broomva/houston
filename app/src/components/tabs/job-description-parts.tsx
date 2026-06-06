import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Button,
  EmptyDescription,
  EmptyHeader,
  EmptyTitle,
  cn,
} from "@houston-ai/core";
import { FileText, Sparkles } from "lucide-react";

export type SubTab = "instructions" | "skills" | "learnings";

/** Slug of the bundled skill the "Help me write this" button invokes. Seeded
 * into every agent by the engine (`houston-agent-files::builtin_skills`). */
export const JOB_DESCRIPTION_SKILL_SLUG = "write-my-job-description";

type SaveState = "idle" | "saving" | "saved";

export interface InstructionsContentLabels {
  emptyTitle: string;
  emptyDescription: string;
  writeButton: string;
  // Assist copy is only rendered when `onWriteWithHouston` is set, so consumers
  // that don't offer the guided writer (e.g. workspace shared context) may omit
  // these — the component falls back to the agent-namespace translations.
  assistButton?: string;
  writeMyself?: string;
  assistHint?: string;
  helper: string;
  saving: string;
  saved: string;
  placeholder: string;
}

export function InstructionsContent({
  content,
  onSave,
  onWriteWithHouston,
  labels,
}: {
  content: string;
  onSave: (content: string) => Promise<unknown>;
  /** Open the agent's chat to the side, pre-loaded with the guided
   * job-description writer. When omitted, only the plain editor is offered. */
  onWriteWithHouston?: () => void;
  labels?: InstructionsContentLabels;
}) {
  const { t } = useTranslation("agents");
  const resolved: InstructionsContentLabels = labels ?? {
    emptyTitle: t("instructions.emptyTitle"),
    emptyDescription: t("instructions.emptyDescription"),
    writeButton: t("instructions.writeButton"),
    assistButton: t("instructions.assistButton"),
    writeMyself: t("instructions.writeMyself"),
    assistHint: t("instructions.assistHint"),
    helper: t("instructions.helper"),
    saving: t("instructions.saving"),
    saved: t("instructions.saved"),
    placeholder: t("instructions.placeholder"),
  };
  // Assist copy may be omitted by `labels`-overriding consumers — fall back to
  // the agent translations so it's always present when the writer is offered.
  const assistButton = resolved.assistButton ?? t("instructions.assistButton");
  const writeMyself = resolved.writeMyself ?? t("instructions.writeMyself");
  const assistHint = resolved.assistHint ?? t("instructions.assistHint");
  const [value, setValue] = useState(content);
  const [editing, setEditing] = useState(false);
  const [state, setState] = useState<SaveState>("idle");

  useEffect(() => {
    setValue(content);
  }, [content]);

  const textareaRef = useCallback(
    (el: HTMLTextAreaElement | null) => {
      if (el && editing) el.focus();
    },
    [editing],
  );

  const handleBlur = async () => {
    if (value === content) return;
    setState("saving");
    await onSave(value);
    setState("saved");
    window.setTimeout(() => setState("idle"), 2000);
  };

  if (!value.trim() && !editing) {
    return (
      <div className="mx-auto max-w-md flex flex-col items-center gap-5 text-center pt-24 px-6">
        <EmptyHeader>
          <EmptyTitle>{resolved.emptyTitle}</EmptyTitle>
          <EmptyDescription>{resolved.emptyDescription}</EmptyDescription>
        </EmptyHeader>
        {onWriteWithHouston ? (
          <div className="flex flex-col items-center gap-3">
            <Button onClick={onWriteWithHouston}>
              <Sparkles className="size-4" />
              {assistButton}
            </Button>
            <button
              type="button"
              onClick={() => setEditing(true)}
              className="text-xs text-muted-foreground hover:text-foreground transition-colors"
            >
              {writeMyself}
            </button>
          </div>
        ) : (
          <Button onClick={() => setEditing(true)}>
            <FileText className="size-4" />
            {resolved.writeButton}
          </Button>
        )}
      </div>
    );
  }

  return (
    <div className="max-w-3xl mx-auto w-full px-6 pb-12 pt-2">
      <div className="flex items-baseline justify-between gap-4 mb-4">
        <p className="text-xs text-muted-foreground max-w-md">
          {resolved.helper}
        </p>
        <div className="flex items-center gap-3 shrink-0">
          {onWriteWithHouston && (
            <button
              type="button"
              onClick={onWriteWithHouston}
              className="inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
            >
              <Sparkles className="size-3.5" />
              {assistHint}
            </button>
          )}
          <span
            className={cn(
              "text-[11px] tabular-nums transition-opacity duration-200",
              state === "idle" ? "opacity-0" : "opacity-100 text-muted-foreground",
            )}
            aria-live="polite"
          >
            {state === "saving" ? resolved.saving : state === "saved" ? resolved.saved : ""}
          </span>
        </div>
      </div>
      <section className="rounded-xl bg-secondary p-3">
        <textarea
          ref={textareaRef}
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onBlur={handleBlur}
          placeholder={resolved.placeholder}
          rows={Math.max(12, value.split("\n").length + 2)}
          className={cn(
            "w-full px-4 py-3 text-sm text-foreground leading-relaxed",
            "placeholder:text-muted-foreground/60",
            "bg-background border border-black/[0.04] rounded-lg",
            "outline-none resize-none transition-shadow duration-200",
            "focus:shadow-[0_1px_2px_rgba(0,0,0,0.04)]",
          )}
        />
      </section>
    </div>
  );
}
