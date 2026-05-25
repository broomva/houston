import type { ReactNode } from "react";
import type {
  AttachmentRejection,
  ChatComposerLabels,
  PrepareAttachments,
} from "./chat-panel-types";
import type { SlashSkillOption } from "./slash-skills";
import type { QueuedChatMessage, QueuedMessageLabels } from "./queued-message-list";

type InputStatus = "ready" | "streaming" | "submitted";

export interface ChatInputProps {
  value?: string;
  onValueChange?: (value: string) => void;
  attachments?: File[];
  onAttachmentsChange?: (files: File[]) => void;
  onSend: (text: string, files: File[]) => void | Promise<void>;
  onStop?: () => void;
  status?: InputStatus;
  placeholder?: string;
  onNotice?: (message: string) => void;
  prepareAttachments?: PrepareAttachments;
  onAttachmentRejections?: (rejections: AttachmentRejection[]) => void;
  footer?: ReactNode;
  header?: ReactNode;
  attachMenu?:
    | ReactNode
    | ((api: { openFilePicker: () => void; close: () => void }) => ReactNode);
  queuedMessages?: QueuedChatMessage[];
  onRemoveQueuedMessage?: (id: string) => void;
  queuedLabels?: QueuedMessageLabels;
  canSendEmpty?: boolean;
  slashSkillOptions?: SlashSkillOption[];
  onSlashSkillSelect?: (skill: SlashSkillOption) => void;
  labels?: ChatComposerLabels;
}
