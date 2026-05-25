export interface SlashSkillOption {
  id: string;
  name: string;
  description: string;
  sourceLabel?: string;
  readonly?: boolean;
}

export function getSlashSkillQuery(text: string): string | null {
  const match = text.match(/(?:^|\n)\/([^\s/]*)$/);
  return match ? match[1].toLowerCase() : null;
}

export function applySlashSkillSelection(text: string): string {
  return text.replace(/(^|\n)\/[^\s/]*$/, "$1");
}

export function filterSlashSkillOptions(
  options: SlashSkillOption[],
  query: string,
): SlashSkillOption[] {
  const q = query.trim().toLowerCase();
  return options
    .filter((option) => {
      if (!q) return true;
      return (
        option.name.toLowerCase().includes(q) ||
        option.description.toLowerCase().includes(q) ||
        option.sourceLabel?.toLowerCase().includes(q)
      );
    })
    .slice(0, 30);
}
