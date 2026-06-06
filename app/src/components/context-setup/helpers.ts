import type { TFunction } from "i18next";
import type {
  ContextSlot,
  ImportSourceKind,
  ResidualQuestion,
} from "@houston-ai/engine-client";

/** How each source kind gathers its path. */
export type SourcePicker = "folder" | "file" | "default";

export interface SourceKindConfig {
  kind: ImportSourceKind;
  picker: SourcePicker;
  /** Pre-filled path for "default" pickers (no dialog needed). */
  defaultPath?: string;
}

/** The source kinds offered, in display order. */
export const SOURCE_KINDS: readonly SourceKindConfig[] = [
  { kind: "localFolder", picker: "folder" },
  { kind: "claudeHome", picker: "default", defaultPath: "~/.claude" },
  { kind: "obsidianVault", picker: "folder" },
  { kind: "chatGptExport", picker: "file" },
  { kind: "claudeAiExport", picker: "file" },
] as const;

/**
 * Questions asked when the user skips import entirely (no corpus to synthesize
 * from). Mirrors the adaptive set the model would otherwise generate, including
 * one source-hint so the user can still be guided to richer material.
 */
export function buildFixedQuestions(
  t: TFunction<"contextSetup">,
): ResidualQuestion[] {
  return [
    { id: "role", prompt: t("fixedQuestions.role"), slot: "user", kind: "content" },
    { id: "company", prompt: t("fixedQuestions.company"), slot: "workspace", kind: "content" },
    { id: "goals", prompt: t("fixedQuestions.goals"), slot: "user", kind: "content" },
    { id: "workStyle", prompt: t("fixedQuestions.workStyle"), slot: "user", kind: "content" },
    { id: "keyPeople", prompt: t("fixedQuestions.keyPeople"), slot: "workspace", kind: "content" },
    { id: "recurringTasks", prompt: t("fixedQuestions.recurringTasks"), slot: "user", kind: "content" },
    { id: "richerSource", prompt: t("fixedQuestions.richerSource"), slot: "workspace", kind: "sourceHint" },
  ];
}

/**
 * Fold answered content questions for one slot into its document. Source-hint
 * questions are excluded — those re-enter the import loop, they are not text.
 */
export function mergeAnswers(
  base: string,
  slot: ContextSlot,
  questions: ResidualQuestion[],
  answers: Record<string, string>,
): string {
  let out = base.trim();
  for (const q of questions) {
    if (q.kind !== "content" || q.slot !== slot) continue;
    const answer = (answers[q.id] ?? "").trim();
    if (!answer) continue;
    out += `${out ? "\n\n" : ""}${q.prompt}\n${answer}`;
  }
  return out;
}
