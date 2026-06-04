import type { ComponentType } from "react";
import type { TabProps } from "../lib/types";
import type { AgentTab } from "./standard-tabs";
import BoardTab from "../components/tabs/board-tab";
import ArchivedTab from "../components/tabs/archived-tab";
import FilesTab from "../components/tabs/files-tab";
import IntegrationsTab from "../components/tabs/integrations-tab";
import JobDescriptionTab from "../components/tabs/job-description-tab";
import RoutinesTab from "../components/tabs/routines-tab";
// Fork-only feature-flagged power-user tabs (advanced.git_panel / .timeline /
// .checkpoints). Upstream #291 dropped the per-agent custom-tab pipeline; these
// three survive because they are flag-gated extras layered on the standard set,
// not per-agent declarations. The other former custom tabs (events/configure/
// prompts/learnings/skills/config) were retired with that refactor.
import GitTab from "../components/tabs/git-tab";
import TimelineTab from "../components/tabs/timeline-tab";
import CheckpointsTab from "../components/tabs/checkpoints-tab";

const BUILTIN_TABS: Record<string, ComponentType<TabProps>> = {
  board: BoardTab,
  archived: ArchivedTab,
  files: FilesTab,
  integrations: IntegrationsTab,
  "job-description": JobDescriptionTab,
  routines: RoutinesTab,
  // Fork flag-gated extras (see import note above).
  git: GitTab,
  timeline: TimelineTab,
  checkpoints: CheckpointsTab,
};

export function resolveTabComponent(tab: AgentTab): ComponentType<TabProps> {
  const Component = BUILTIN_TABS[tab.builtIn];
  if (!Component) {
    throw new Error(`Unknown built-in tab: ${tab.builtIn}`);
  }
  return Component;
}
