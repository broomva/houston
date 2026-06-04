import { cn, Tooltip, TooltipContent, TooltipTrigger } from "@houston-ai/core";
import { Check, Minus } from "lucide-react";

interface SelectAllButtonProps {
  /** Every item in the section is selected. */
  checked: boolean;
  /** Some — but not all — items are selected. */
  indeterminate: boolean;
  /** Toggle the whole section's selection. */
  onToggle: () => void;
  /** Accessible label / tooltip (e.g. "Select all"). */
  label: string;
}

/**
 * Tri-state "select all" checkbox for a board column header. Filled with a
 * check when the whole section is selected, a dash when partially selected.
 * Shown in the Needs you header once a needs-you selection is active so the
 * user can grab (or clear) the whole section in one click.
 */
export function SelectAllButton({
  checked,
  indeterminate,
  onToggle,
  label,
}: SelectAllButtonProps) {
  const active = checked || indeterminate;
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          role="checkbox"
          aria-checked={indeterminate ? "mixed" : checked}
          aria-label={label}
          onClick={(e) => {
            e.stopPropagation();
            onToggle();
          }}
          className={cn(
            "size-4 rounded-[5px] border flex items-center justify-center transition-colors",
            active
              ? "bg-primary border-primary text-primary-foreground"
              : "border-muted-foreground/40 text-transparent hover:border-foreground",
          )}
        >
          {checked ? (
            <Check className="size-3" strokeWidth={3} />
          ) : indeterminate ? (
            <Minus className="size-3" strokeWidth={3} />
          ) : null}
        </button>
      </TooltipTrigger>
      <TooltipContent side="top">{label}</TooltipContent>
    </Tooltip>
  );
}
