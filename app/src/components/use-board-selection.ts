import { useCallback, useEffect, useState } from "react";
import type { KanbanItem } from "@houston-ai/board";
import { useBulkUpdateActivity, useBulkDeleteActivity } from "../hooks/queries";
import { ARCHIVED_STATUS, toggleAllIds } from "../lib/mission-selection";

/**
 * Per-agent multi-select state + bulk action handlers for the board tab.
 *
 * Owns the set of selected mission ids and the move/archive/delete
 * mutations that operate on it. The selection resets whenever `resetKey`
 * changes (the BoardTab instance is reused across agents, so selection
 * must not bleed from one agent to the next). Bulk actions clear the
 * selection on success; failures propagate to the caller's mutation so a
 * toast surfaces (no silent swallow).
 */
export function useBoardSelection(
  agentPath: string | undefined,
  resetKey: string,
) {
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const bulkUpdate = useBulkUpdateActivity(agentPath);
  const bulkDelete = useBulkDeleteActivity(agentPath);

  useEffect(() => {
    setSelectedIds(new Set());
  }, [resetKey]);

  const toggle = useCallback((item: KanbanItem) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(item.id)) next.delete(item.id);
      else next.add(item.id);
      return next;
    });
  }, []);

  const clear = useCallback(() => setSelectedIds(new Set()), []);

  /** Select-all for a section: if every id is already selected, clear them;
   *  otherwise add them all. */
  const toggleAll = useCallback((ids: string[]) => {
    setSelectedIds((prev) => toggleAllIds(prev, ids));
  }, []);

  const move = useCallback(
    async (status: string) => {
      await bulkUpdate.mutateAsync({ ids: Array.from(selectedIds), update: { status } });
      clear();
    },
    [bulkUpdate, selectedIds, clear],
  );

  const archive = useCallback(async () => {
    await bulkUpdate.mutateAsync({
      ids: Array.from(selectedIds),
      update: { status: ARCHIVED_STATUS },
    });
    clear();
  }, [bulkUpdate, selectedIds, clear]);

  const remove = useCallback(async () => {
    await bulkDelete.mutateAsync(Array.from(selectedIds));
    clear();
  }, [bulkDelete, selectedIds, clear]);

  /** Archive an explicit id list (used by the Done column "archive all"),
   *  independent of the current selection. */
  const archiveIds = useCallback(
    async (ids: string[]) => {
      await bulkUpdate.mutateAsync({ ids, update: { status: ARCHIVED_STATUS } });
    },
    [bulkUpdate],
  );

  return { selectedIds, toggle, toggleAll, clear, move, archive, remove, archiveIds };
}
