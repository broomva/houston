---
name: write-my-job-description
description: Draft this agent's job description through a short guided conversation, then save it.
version: 1
tags: [setup, getting-started]
category: setup
image: writing-hand
---

## What this does

Help the user write this agent's **job description** — the standing instructions
that shape how this agent works. The user reached this from the empty Job
Description screen, so the job description is currently blank and a blank page
feels daunting. Replace the blank page with a short, friendly conversation that
ends with a real job description saved for them.

## Procedure

1. **Open with a simple starter, not a blank page.** From this agent's name and
   anything already known about it, propose a short plain-language starter (3–5
   sentences) the user can react to. Frame it as a starting point, e.g. "Here's a
   simple starting point for what I do — tell me what to change."

2. **Interview briefly — a few questions, one at a time.** Ask only what you need:
   - What should I focus on day to day?
   - How should I sound — brief and direct, or warm and detailed?
   - Should I check with you before acting, or just go ahead?
   - Anything I should always do, or never do?
   Skip anything the user already covered in the starter. Two or three questions
   is usually enough — never dump a form.

3. **Write the job description.** When you have enough, write the final text to
   this agent's `CLAUDE.md` at the agent root (that file *is* the job
   description). Use a short role line plus a few clear sections (focus, style,
   how to work). Keep it concrete and human.

4. **Confirm and set expectations.** Tell the user it's saved, summarize what
   changed in one sentence, and let them know the new instructions take effect
   the next time they chat with you. Invite tweaks: "Want me to adjust anything?"

## Rules

- Never show the user file names, paths, or settings. Speak about "your job
  description," never "CLAUDE.md."
- Keep it short and human — a founder should read the whole thing in 20 seconds.
- One question at a time. No multi-question forms.
- If the user hands you everything up front, skip the interview and just write it.
