import { useCallback } from "react";
import type { ChatInputProps } from "./chat-input-types";
import { useSlashSkillPicker } from "./chat-slash-skills";
import type { PromptInputMessage } from "./ai-elements/prompt-input";
import {
  PromptInput,
  PromptInputBody,
  PromptInputHeader,
  PromptInputTextarea,
} from "./ai-elements/prompt-input";
import { ComposerTrailing } from "./attachment-chip";
import {
  ChatInputAttachButton,
  ChatInputAttachments,
} from "./chat-input-attachments";
import { QueuedMessageList } from "./queued-message-list";
import { useControllable } from "./use-file-drop-zone";
import { useComposerAttachments } from "./use-composer-attachments";

export type { ChatComposerLabels } from "./chat-panel-types";
export type { ChatInputProps } from "./chat-input-types";

export function ChatInput({
  value,
  onValueChange,
  attachments,
  onAttachmentsChange,
  onSend,
  onStop,
  status = "ready",
  placeholder = "Type a message...",
  onNotice,
  prepareAttachments,
  onAttachmentRejections,
  footer,
  header,
  attachMenu,
  queuedMessages = [],
  onRemoveQueuedMessage,
  queuedLabels,
  canSendEmpty = false,
  slashSkillOptions = [],
  onSlashSkillSelect,
  labels,
}: ChatInputProps) {
  const [text, setText] = useControllable(value, onValueChange, "");
  const isTextControlled = value !== undefined;
  const {
    files,
    setFiles,
    isFilesControlled,
    fileInputRef,
    handleFileChange,
    handlePaste,
    openFilePicker,
    removeFile,
  } = useComposerAttachments({
    attachments,
    onAttachmentsChange,
    prepareAttachments,
    onAttachmentRejections,
    onNotice,
    labels,
  });
  const slashSkillPicker = useSlashSkillPicker({
    text,
    setText,
    options: slashSkillOptions,
    onSelect: onSlashSkillSelect,
    labels,
  });

  const handleTextChange = useCallback(
    (e: React.ChangeEvent<HTMLTextAreaElement>) => {
      setText(e.target.value);
      slashSkillPicker.clearDismissal();
    },
    [setText, slashSkillPicker],
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (slashSkillPicker.handleKeyDown(e)) return;
      if (e.key === "Escape" && status !== "ready" && onStop) {
        e.preventDefault();
        onStop();
      }
    },
    [slashSkillPicker, status, onStop],
  );

  const handleSubmit = useCallback(
    async (message: PromptInputMessage) => {
      const trimmed = message.text?.trim();
      if (!trimmed && files.length === 0 && !canSendEmpty) return;
      await onSend(trimmed ?? "", files);
      // In uncontrolled mode, clear our own state. In controlled mode the
      // parent is responsible for clearing.
      if (!isTextControlled) setText("");
      if (!isFilesControlled) setFiles([]);
    },
    [onSend, files, canSendEmpty, isTextControlled, isFilesControlled, setText, setFiles],
  );

  const hasContent = canSendEmpty || text.trim().length > 0 || files.length > 0;

  return (
    <div className="shrink-0 px-4 pb-6 pt-2">
      <div className="max-w-3xl mx-auto relative">
        <ChatInputAttachments
          fileInputRef={fileInputRef}
          files={files}
          onFileChange={handleFileChange}
          onRemoveFile={removeFile}
        />

        <QueuedMessageList
          messages={queuedMessages}
          onRemove={onRemoveQueuedMessage}
          labels={queuedLabels}
        />

        {slashSkillPicker.menu}

        <PromptInput onSubmit={handleSubmit}>
          {header && (
            <PromptInputHeader className="pb-1">
              {header}
            </PromptInputHeader>
          )}

          <ChatInputAttachButton
            onOpenFilePicker={openFilePicker}
            attachMenu={attachMenu}
          />

          <PromptInputBody>
            <PromptInputTextarea
              onChange={handleTextChange}
              onKeyDown={handleKeyDown}
              onPaste={handlePaste}
              value={text}
              placeholder={placeholder}
            />
          </PromptInputBody>

          <ComposerTrailing
            status={status}
            hasContent={hasContent}
            onStop={onStop}
          />
        </PromptInput>

        {footer && (
          <div className="flex items-center px-2.5 pt-1">
            {footer}
          </div>
        )}
      </div>
    </div>
  );
}
