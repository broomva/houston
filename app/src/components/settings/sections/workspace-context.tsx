import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@houston-ai/core";
import { Sparkles } from "lucide-react";
import { useWorkspaceStore } from "../../../stores/workspaces";
import {
  useSaveWorkspaceContext,
  useWorkspaceContext,
} from "../../../hooks/queries/use-workspace-context";
import {
  InstructionsContent,
  type InstructionsContentLabels,
} from "../../tabs/job-description-parts";
import { ContextSetupDialog, useLastUsedProvider } from "../../context-setup";

type Slot = "workspace" | "user";

/**
 * "Set up automatically" / "Improve with my content" — opens the import +
 * synthesize wizard. Shown above both shared-context editors so the user can
 * reach it whether they are viewing the workspace or the user document.
 */
function ContextSetupLauncher() {
  const { t } = useTranslation("contextSetup");
  const current = useWorkspaceStore((s) => s.current);
  const { data } = useWorkspaceContext(current?.id);
  const [open, setOpen] = useState(false);
  const provider = useLastUsedProvider();

  if (!current) return null;
  const isEmpty = !data?.user?.trim() && !data?.workspace?.trim();

  return (
    <div className="max-w-3xl mx-auto w-full px-6 pt-4 flex justify-end">
      <Button variant="secondary" onClick={() => setOpen(true)}>
        <Sparkles className="size-4" />
        {isEmpty ? t("launcher.setUp") : t("launcher.improve")}
      </Button>
      <ContextSetupDialog
        open={open}
        onOpenChange={setOpen}
        workspaceId={current.id}
        provider={provider}
      />
    </div>
  );
}

function useSlotEditor(slot: Slot) {
  const currentWorkspace = useWorkspaceStore((s) => s.current);
  const { data } = useWorkspaceContext(currentWorkspace?.id);
  const save = useSaveWorkspaceContext(currentWorkspace?.id);

  const content = data?.[slot] ?? "";

  const onSave = async (next: string) => {
    if (!data) return;
    await save.mutateAsync({ ...data, [slot]: next });
  };

  return { ready: !!currentWorkspace && !!data, content, onSave };
}

function useSlotLabels(prefix: "workspaceContext" | "userContext"): InstructionsContentLabels {
  const { t } = useTranslation("settings");
  return {
    emptyTitle: t(`${prefix}.emptyTitle`),
    emptyDescription: t(`${prefix}.emptyDescription`),
    writeButton: t(`${prefix}.writeButton`),
    helper: t(`${prefix}.helper`),
    saving: t(`${prefix}.saving`),
    saved: t(`${prefix}.saved`),
    placeholder: t(`${prefix}.placeholder`),
  };
}

export function WorkspaceContextSection() {
  const editor = useSlotEditor("workspace");
  const labels = useSlotLabels("workspaceContext");
  if (!editor.ready) return null;
  return (
    <>
      <ContextSetupLauncher />
      <InstructionsContent
        content={editor.content}
        onSave={editor.onSave}
        labels={labels}
      />
    </>
  );
}

export function UserContextSection() {
  const editor = useSlotEditor("user");
  const labels = useSlotLabels("userContext");
  if (!editor.ready) return null;
  return (
    <>
      <ContextSetupLauncher />
      <InstructionsContent
        content={editor.content}
        onSave={editor.onSave}
        labels={labels}
      />
    </>
  );
}
