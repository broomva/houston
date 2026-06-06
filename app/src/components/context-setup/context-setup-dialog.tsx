import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@houston-ai/core";
import { Loader2 } from "lucide-react";
import type {
  ImportSource,
  ImportSummary,
  ResidualQuestion,
} from "@houston-ai/engine-client";
import { tauriWorkspaces } from "../../lib/tauri";
import { useSaveWorkspaceContext } from "../../hooks/queries/use-workspace-context";
import { SourceChooser } from "./source-chooser";
import { DraftReview } from "./draft-review";
import { buildFixedQuestions, mergeAnswers } from "./helpers";

type Step = "sources" | "importing" | "synthesizing" | "review" | "done";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  workspaceId: string;
  /** Workspace provider id (e.g. "anthropic"). Falls back to engine default. */
  provider?: string;
}

export function ContextSetupDialog({
  open,
  onOpenChange,
  workspaceId,
  provider,
}: Props) {
  const { t } = useTranslation("contextSetup");
  const save = useSaveWorkspaceContext(workspaceId);

  const [step, setStep] = useState<Step>("sources");
  const [sources, setSources] = useState<ImportSource[]>([]);
  const [summary, setSummary] = useState<ImportSummary | null>(null);
  const [questions, setQuestions] = useState<ResidualQuestion[]>([]);
  const [userText, setUserText] = useState("");
  const [workspaceText, setWorkspaceText] = useState("");
  const [answers, setAnswers] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState(false);

  const reset = () => {
    setStep("sources");
    setSources([]);
    setSummary(null);
    setQuestions([]);
    setUserText("");
    setWorkspaceText("");
    setAnswers({});
    setSaving(false);
  };

  const handleOpenChange = (next: boolean) => {
    if (!next) reset();
    onOpenChange(next);
  };

  const addSource = (src: ImportSource): ImportSource[] => {
    const exists = sources.some((s) => s.kind === src.kind && s.path === src.path);
    const next = exists ? sources : [...sources, src];
    setSources(next);
    return next;
  };

  // Import + synthesize over the given source list. Errors are surfaced as
  // toasts by the tauri call() wrapper; we just fall back to the chooser.
  const runPipeline = async (srcList: ImportSource[]) => {
    setStep("importing");
    try {
      const sum = await tauriWorkspaces.importContext(workspaceId, { sources: srcList });
      setSummary(sum);
      setStep("synthesizing");
      const draft = await tauriWorkspaces.synthesizeContext(workspaceId, { provider });
      setUserText(draft.user);
      setWorkspaceText(draft.workspace);
      setQuestions(draft.questions);
      setStep("review");
    } catch {
      setStep("sources");
    }
  };

  const handleSkip = () => {
    setSummary(null);
    setQuestions(buildFixedQuestions(t));
    setUserText("");
    setWorkspaceText("");
    setStep("review");
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      const user = mergeAnswers(userText, "user", questions, answers);
      const workspace = mergeAnswers(workspaceText, "workspace", questions, answers);
      await save.mutateAsync({ user, workspace });
      setStep("done");
    } catch {
      // toasted by the setContext call wrapper; stay on review to retry.
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-2xl max-h-[88vh] flex flex-col gap-0 p-0 overflow-hidden">
        <DialogHeader className="shrink-0 px-6 pt-6 pb-3">
          <DialogTitle>{t("title")}</DialogTitle>
          <DialogDescription>{t("subtitle")}</DialogDescription>
        </DialogHeader>

        <div className="flex-1 min-h-0 overflow-y-auto px-6 pb-6">
          {step === "sources" && (
            <SourceChooser
              sources={sources}
              onAddSource={(s) => void addSource(s)}
              onRemoveSource={(i) => setSources(sources.filter((_, idx) => idx !== i))}
              onContinue={() => void runPipeline(sources)}
              onSkip={handleSkip}
            />
          )}

          {(step === "importing" || step === "synthesizing") && (
            <Pending
              heading={t(step === "importing" ? "importing.heading" : "synthesizing.heading")}
              status={step === "synthesizing" ? t("synthesizing.status") : undefined}
            />
          )}

          {step === "review" && (
            <div className="flex flex-col gap-4">
              {summary && (summary.skipped.length > 0 || summary.truncated) && (
                <p className="text-[11px] text-muted-foreground">
                  {summary.skipped.length > 0 &&
                    t("importing.skipped", { count: summary.skipped.length })}{" "}
                  {summary.truncated && t("importing.truncated")}
                </p>
              )}
              <DraftReview
                userText={userText}
                workspaceText={workspaceText}
                onUserChange={setUserText}
                onWorkspaceChange={setWorkspaceText}
                questions={questions}
                answers={answers}
                onAnswer={(id, value) => setAnswers((a) => ({ ...a, [id]: value }))}
                onAddSource={(s) => void runPipeline(addSource(s))}
                onSave={() => void handleSave()}
                saving={saving}
              />
            </div>
          )}

          {step === "done" && (
            <div className="flex flex-col items-center gap-4 text-center pt-10 pb-6 px-6">
              <h3 className="text-base font-semibold text-foreground">
                {t("done.heading")}
              </h3>
              <p className="text-sm text-muted-foreground max-w-sm">
                {t("done.description")}
              </p>
              <Button onClick={() => handleOpenChange(false)}>{t("done.close")}</Button>
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}

function Pending({ heading, status }: { heading: string; status?: string }) {
  return (
    <div className="flex flex-col items-center gap-3 text-center pt-16 pb-10 px-6">
      <Loader2 className="size-6 animate-spin text-muted-foreground" />
      <h3 className="text-sm font-semibold text-foreground">{heading}</h3>
      {status && <p className="text-xs text-muted-foreground max-w-sm">{status}</p>}
    </div>
  );
}
