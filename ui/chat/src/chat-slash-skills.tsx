import { useCallback, useEffect, useMemo, useState } from "react";
import { cn } from "@houston-ai/core";
import type { ChatComposerLabels } from "./chat-panel-types";
import type { SlashSkillOption } from "./slash-skills";
import {
  applySlashSkillSelection,
  filterSlashSkillOptions,
  getSlashSkillQuery,
} from "./slash-skills";

interface UseSlashSkillPickerArgs {
  text: string;
  setText: (value: string) => void;
  options: SlashSkillOption[];
  onSelect?: (skill: SlashSkillOption) => void;
  labels?: ChatComposerLabels;
}

export function useSlashSkillPicker({
  text,
  setText,
  options,
  onSelect,
  labels,
}: UseSlashSkillPickerArgs) {
  const [activeIndex, setActiveIndex] = useState(0);
  const [dismissedQuery, setDismissedQuery] = useState<string | null>(null);
  const query = useMemo(() => getSlashSkillQuery(text), [text]);
  const matches = useMemo(
    () => (query === null ? [] : filterSlashSkillOptions(options, query)),
    [options, query],
  );
  const open = query !== null && dismissedQuery !== query && options.length > 0;

  useEffect(() => {
    setActiveIndex(0);
  }, [query, matches.length]);

  const clearDismissal = useCallback(() => setDismissedQuery(null), []);

  const select = useCallback(
    (skill: SlashSkillOption) => {
      setText(applySlashSkillSelection(text));
      setDismissedQuery(null);
      onSelect?.(skill);
    },
    [onSelect, setText, text],
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>): boolean => {
      if (!open) return false;
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setActiveIndex((idx) => (idx + 1) % Math.max(matches.length, 1));
        return true;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setActiveIndex(
          (idx) =>
            (idx - 1 + Math.max(matches.length, 1)) %
            Math.max(matches.length, 1),
        );
        return true;
      }
      if ((e.key === "Enter" || e.key === "Tab") && matches[activeIndex]) {
        e.preventDefault();
        select(matches[activeIndex]);
        return true;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        setDismissedQuery(query);
        return true;
      }
      return false;
    },
    [activeIndex, matches, open, query, select],
  );

  const menu = open ? (
    <SlashSkillMenu
      matches={matches}
      activeIndex={activeIndex}
      labels={labels}
      onSelect={select}
    />
  ) : null;

  return { clearDismissal, handleKeyDown, menu };
}

function SlashSkillMenu({
  matches,
  activeIndex,
  labels,
  onSelect,
}: {
  matches: SlashSkillOption[];
  activeIndex: number;
  labels?: ChatComposerLabels;
  onSelect: (skill: SlashSkillOption) => void;
}) {
  return (
    <div className="absolute bottom-[calc(100%+0.5rem)] left-0 right-0 z-20 max-h-[320px] overflow-y-auto rounded-xl border border-border bg-popover p-1.5 shadow-xl">
      {matches.length > 0 ? (
        matches.map((skill, idx) => (
          <button
            key={skill.id}
            type="button"
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => onSelect(skill)}
            className={cn(
              "flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left transition-colors",
              idx === activeIndex
                ? "bg-accent text-accent-foreground"
                : "text-foreground hover:bg-accent/70",
            )}
          >
            <span className="shrink-0 text-lg text-muted-foreground">/</span>
            <span className="min-w-0 flex-1">
              <span className="flex min-w-0 items-baseline gap-2">
                <span className="truncate text-sm font-medium">{skill.name}</span>
                {skill.sourceLabel && (
                  <span className="shrink-0 text-xs text-muted-foreground">
                    {skill.sourceLabel}
                  </span>
                )}
              </span>
              {skill.description && (
                <span className="block truncate text-xs text-muted-foreground">
                  {skill.description}
                </span>
              )}
            </span>
          </button>
        ))
      ) : (
        <div className="px-3 py-2 text-sm text-muted-foreground">
          {labels?.slashSkillEmpty ?? "No matching skills"}
        </div>
      )}
    </div>
  );
}
