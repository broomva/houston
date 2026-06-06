import { useTranslation } from "react-i18next";
import { Button, cn } from "@houston-ai/core";
import { FolderSearch } from "lucide-react";
import type { ImportSource, ResidualQuestion } from "@houston-ai/engine-client";
import { osPickDirectory } from "../../lib/os-bridge";

const TEXTAREA_CLASS = cn(
  "w-full px-4 py-3 text-sm text-foreground leading-relaxed",
  "placeholder:text-muted-foreground/60",
  "bg-background border border-black/[0.04] rounded-lg",
  "outline-none resize-none transition-shadow duration-200",
  "focus:shadow-[0_1px_2px_rgba(0,0,0,0.04)]",
);

interface Props {
  userText: string;
  workspaceText: string;
  onUserChange: (v: string) => void;
  onWorkspaceChange: (v: string) => void;
  questions: ResidualQuestion[];
  answers: Record<string, string>;
  onAnswer: (id: string, value: string) => void;
  onAddSource: (src: ImportSource) => void;
  onSave: () => void;
  saving: boolean;
}

export function DraftReview({
  userText,
  workspaceText,
  onUserChange,
  onWorkspaceChange,
  questions,
  answers,
  onAnswer,
  onAddSource,
  onSave,
  saving,
}: Props) {
  const { t } = useTranslation("contextSetup");

  const pickRicherSource = async () => {
    const path = await osPickDirectory();
    if (path) onAddSource({ kind: "localFolder", path });
  };

  return (
    <div className="flex flex-col gap-5">
      <div className="flex flex-col gap-1">
        <h3 className="text-sm font-semibold text-foreground">
          {t("review.heading")}
        </h3>
        <p className="text-xs text-muted-foreground">{t("review.description")}</p>
      </div>

      <label className="flex flex-col gap-1.5">
        <span className="text-xs font-medium text-foreground">
          {t("review.userLabel")}
        </span>
        <textarea
          value={userText}
          onChange={(e) => onUserChange(e.target.value)}
          placeholder={t("review.userPlaceholder")}
          rows={Math.max(4, userText.split("\n").length + 1)}
          className={TEXTAREA_CLASS}
        />
      </label>

      <label className="flex flex-col gap-1.5">
        <span className="text-xs font-medium text-foreground">
          {t("review.workspaceLabel")}
        </span>
        <textarea
          value={workspaceText}
          onChange={(e) => onWorkspaceChange(e.target.value)}
          placeholder={t("review.workspacePlaceholder")}
          rows={Math.max(4, workspaceText.split("\n").length + 1)}
          className={TEXTAREA_CLASS}
        />
      </label>

      {questions.length > 0 && (
        <div className="flex flex-col gap-3">
          <span className="text-xs font-medium text-foreground">
            {t("review.questionsHeading")}
          </span>
          {questions.map((q) =>
            q.kind === "sourceHint" ? (
              <div key={q.id} className="flex flex-col gap-1.5">
                <span className="text-xs text-muted-foreground">{q.prompt}</span>
                <Button variant="secondary" onClick={() => void pickRicherSource()}>
                  <FolderSearch className="size-4" />
                  {t("review.sourceHintFolder")}
                </Button>
              </div>
            ) : (
              <label key={q.id} className="flex flex-col gap-1.5">
                <span className="text-xs text-muted-foreground">{q.prompt}</span>
                <textarea
                  value={answers[q.id] ?? ""}
                  onChange={(e) => onAnswer(q.id, e.target.value)}
                  placeholder={t("review.answerPlaceholder")}
                  rows={2}
                  className={TEXTAREA_CLASS}
                />
              </label>
            ),
          )}
        </div>
      )}

      <div className="flex justify-end pt-1">
        <Button onClick={onSave} disabled={saving}>
          {saving ? t("review.saving") : t("review.save")}
        </Button>
      </div>
    </div>
  );
}
