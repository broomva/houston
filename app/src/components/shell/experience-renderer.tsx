import { Suspense } from "react";
import { Spinner } from "@houston-ai/core";
import { resolveTabComponent } from "../../agents/tab-resolver";
import { STANDARD_TABS, type AgentTab } from "../../agents/standard-tabs";
import type { AgentDefinition, Agent } from "../../lib/types";

interface AgentRendererProps {
  agentDef: AgentDefinition;
  agent: Agent;
  activeTabId: string;
  /**
   * Tab set to render. Defaults to the hardcoded standard set; the shell passes
   * the standard set plus any active flag-gated extras (git/timeline/checkpoints).
   */
  tabs?: readonly AgentTab[];
}

export function AgentRenderer({
  agentDef,
  agent,
  activeTabId,
  tabs = STANDARD_TABS,
}: AgentRendererProps) {
  return (
    <div className="h-full w-full relative min-h-0">
      {tabs.map((tab) => {
        const TabComponent = resolveTabComponent(tab);
        const isActive = tab.id === activeTabId;
        return (
          <div
            key={tab.id}
            className={isActive ? "h-full w-full flex flex-col min-h-0" : "hidden"}
          >
            <Suspense
              fallback={
                <div className="h-full flex items-center justify-center">
                  <Spinner className="size-5" />
                </div>
              }
            >
              <TabComponent agent={agent} agentDef={agentDef} />
            </Suspense>
          </div>
        );
      })}
    </div>
  );
}
