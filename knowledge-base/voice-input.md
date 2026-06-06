# Voice input (phase 2)

Status: **not built.** Design captured so phase 2 starts from prior art, not a blank page.

## Why this doc exists

The instruction-writer feature (Job Description → "Help me write this" → guided
side-chat, see `agent-manifest.md`) shipped text-only. The original ask included
"voice enabled to further fine tune." Voice was deferred so the core writer ships
first, but the architecture is decided here from the bstack knowledge graph
(`/kg voice`) so phase 2 doesn't re-research.

## Decision

Houston is **local-first desktop (Tauri)**. Voice input follows the same ethos:
**on-device speech-to-text, no cloud, no API keys**. Do NOT reach for a cloud STT
service (privacy + cost + offline) and do NOT lean on `webkitSpeechRecognition`
in the WKWebView (inconsistent on macOS; routes audio to Apple servers anyway).

## Prior art (from the KG)

- **`omnivoice-studio`** — open-source, fully-local ElevenLabs alternative.
  Voice cloning/design/dubbing **and real-time dictation**, 646 languages,
  FastAPI on `127.0.0.1:3900`, MPS-accelerated on Apple Silicon, ships its own
  Tauri desktop wrapper + MCP server. Proves a local voice stack co-exists with a
  Tauri app. Standalone skill: `broomva/omnivoice-skill` (MIT). License
  FSL-1.1-ALv2 (free personal/internal; → Apache-2.0 two years post-release).
- **`superwhisper` ecosystem** — the most-cited macOS dictation product. Three
  lessons to port:
  1. **Two-stage split.** Deterministic transcription (on-device Whisper /
     Parakeet) is separate from optional LLM cleanup. The producer has **no LLM**.
  2. **Modes** = (model + system prompt + scope rule) auto-activated by context.
     Maps onto a per-agent "voice mode".
  3. File-watch surface is end-of-utterance only (not streaming). For a live HUD
     you need partials; for composer dictation, end-of-utterance is fine.
- **Code refs**: `zachlatta/freeflow` (open Wispr-Flow alt — the actual code
  reference), `whisper.cpp`, NVIDIA Parakeet. Local Whisper sizes: Nano ~75 MB,
  Standard ~500 MB (free tier), Parakeet V2/V3 ~480 MB English-only (~6% WER).

## Recommended Houston architecture (phase 2)

1. **Local model, downloaded on demand.** Bundle nothing heavy; mirror the
   `houston-claude-installer` pattern (pinned URL + sha256 + atomic install +
   progress events) to fetch a small Whisper / Parakeet model on first use. Keeps
   the installer slim and the feature opt-in.
2. **Deterministic STT producer → composer text.** A mic button on the chat
   composer (`ui/chat`) records, transcribes locally, drops text into the
   controlled composer `value`. No LLM in this stage.
3. **Optional LLM cleanup as a "mode"** (later). Punctuation/cleanup pass, model
   swappable, off by default. Do not block dictation on it.
4. **Boundary**: the recorder/transcriber is engine-side or a sidecar (frontend
   -agnostic, mirrors `omnivoice-studio`'s FastAPI shape); the mic button +
   waveform live in `ui/chat`, props-only, no engine assumptions. The app wires
   them together.

## Where it plugs in

- Composer: `ui/chat/src/chat-input.tsx` (`value` / `onValueChange` already
  controlled — dictation just sets `value`).
- The instruction-writer side-chat (`agent-manifest.md`) is the first consumer:
  speak to fine-tune the job description instead of typing.

## References

- KG: `research/entities/tool/omnivoice-studio.md`,
  `research/entities/project/superwhisper-voice-ecosystem.md`
- `knowledge-base/cli-bundling.md` + `houston-claude-installer` (download pattern)
