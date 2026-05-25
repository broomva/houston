# Design System: Houston
**Project ID:** houston-core-platform

Houston is a high-fidelity agentic terminal platform built for desktop (Tauri 2) and mobile environments. It features a premium, clean, near-black minimalist visual aesthetic tailored to feel capable, calm, and invisible.

---

## 1. Visual Theme & Atmosphere

Houston is designed to be a **quiet expert**. The user interface feels like messaging a brilliant assistant, avoiding dense toolbars, complex configuration overlays, or overly corporate UI structures. 

* **Atmosphere:** Deep, premium, minimalist, and hyper-focused.
* **Density:** Spacious but highly functional (compact, not cramped). Focus is dedicated entirely to the chat workspace and active agent tasks.
* **Visual Polish:** Sleek dark-first elements (supporting light mode), whisper-soft borders, custom conic-gradient borders for active tasks (`card-running-glow`), and smooth micro-animations that communicate constant activity.
* **Core Philosophies:**
  1. **Show, Don't Configure:** Settings panels are minimized; parameters are inferred or presented through single obvious options.
  2. **Always Feel Alive:** When an AI agent is working, visual motion occurs every second (pulsing tools, bouncing loader dots, counting step progress). Silence equals broken.
  3. **Chat-First:** The conversational interface is the primary mechanism; all secondary UI supports this core interaction.
  4. **Non-Technical Copy:** Concepts use plain language. "Prompt" instead of "Description", "Needs You" instead of "In Review".
  5. **Action Accessibility:** Interactive controls are permanently visible. Hover states are used for enrichment, never to gate action accessibility.

---

## 2. Color Palette & Roles

The visual interface is predominantly grayscale, reserving color strictly for active status indicators, agent/channel avatar badges, link highlights, and the rotating task progress gradient.

### Light Mode default tokens (`:root`)

| Semantic Token | Descriptive Name | Hex / RGBA Value | Functional Role |
| :--- | :--- | :--- | :--- |
| `--ht-background` | Pure White | `#ffffff` | Primary app and workspace canvas background. |
| `--ht-foreground` | Near-Black Charcoal | `#0d0d0d` | Primary body text, main headings, and active labels. |
| `--ht-card` | Pure White | `#ffffff` | Background for panels, modal dialogs, and secondary layouts. |
| `--ht-card-fg` | Near-Black Charcoal | `#0d0d0d` | Text inside card components and dashboard panels. |
| `--ht-primary` | Near-Black Charcoal | `#0d0d0d` | Background for primary buttons and high-contrast UI highlights. |
| `--ht-primary-fg` | Pure White | `#ffffff` | Text overlaid on top of primary buttons or badges. |
| `--ht-secondary` | Soft Warm Gray | `#f5f5f5` | Sidebar canvas background and muted controls. |
| `--ht-secondary-fg` | Near-Black Charcoal | `#0d0d0d` | Text within secondary layouts. |
| `--ht-muted` | Soft Light Gray | `#f5f5f5` | Secondary card background and inactive panels. |
| `--ht-muted-fg` | Medium Cool Gray | `#8e8e8e` | Secondary text, placeholder styling, and secondary icons. |
| `--ht-accent` | Whisper Dark Gray | `rgba(0, 0, 0, 0.07)` | Hover and active pressed states for interactive controls. |
| `--ht-accent-fg` | Slate Gray | `#424242` | Accented button labels and highlighted metadata. |
| `--ht-border` | Light Charcoal Border | `rgba(0, 0, 0, 0.06)` | Quiet borders (5% opacity) for layout isolation. |
| `--ht-input` | Subtle Input Gray | `#e3e3e3` | Outer borders for input fields and selectors. |
| `--ht-ring` | Near-Black Ring | `#0d0d0d` | Keyboard focus ring outlines. |
| `--ht-sidebar` | Sidebar Canvas | `#f5f5f5` | Default sidebar background. |
| `--ht-sidebar-fg` | Near-Black Sidebar text | `#0d0d0d` | Sidebar label hierarchy. |
| `--ht-sidebar-border`| Muted Sidebar Divider | `rgba(0, 0, 0, 0.06)` | Vertical sidebar separator lines. |
| `--ht-sidebar-accent`| Sidebar Hover Accent | `rgba(0, 0, 0, 0.07)` | Hover backgrounds for sidebar navigation. |

### Dark Mode overrides (`[data-theme="dark"]`)

| Semantic Token | Descriptive Name | Hex / RGBA Value | Functional Role |
| :--- | :--- | :--- | :--- |
| `--ht-background` | Matte Gray-Black | `#1e1e1e` | Dark mode primary app canvas. |
| `--ht-foreground` | Bright Silver-Gray | `#e5e5e5` | Primary body text and active headings. |
| `--ht-card` | Deep Obsidian | `#282828` | Background for dark mode panels and modals. |
| `--ht-card-fg` | Bright Silver-Gray | `#e5e5e5` | Text inside dark mode cards and dashboard elements. |
| `--ht-primary` | Bright Silver-Gray | `#e5e5e5` | Main CTA background. |
| `--ht-primary-fg` | Obsidian Accent | `#141414` | Text inside primary CTA buttons in dark mode. |
| `--ht-secondary` | Dark Warm Gray | `#252525` | Sidebar canvas background in dark mode. |
| `--ht-secondary-fg` | Bright Silver-Gray | `#e5e5e5` | Muted dark mode labels. |
| `--ht-muted` | Dark Charcoal | `#252525` | Background for secondary dark panels. |
| `--ht-muted-fg` | Muted Silver Gray | `#9a9a9a` | Secondary descriptions and placeholders in dark mode. |
| `--ht-accent` | Whisper White Gray | `rgba(255, 255, 255, 0.08)` | Dark mode hover states. |
| `--ht-accent-fg` | Warm White Accent | `#d4d4d4` | Muted labels and secondary interactive text. |
| `--ht-border` | Dark Muted Border | `rgba(128, 128, 128, 0.10)` | Soft borders for panels in dark mode. |
| `--ht-input` | Dark Input Slate | `#383838` | Input outlines and form backgrounds. |
| `--ht-ring` | Bright Silver Ring | `#e5e5e5` | Focus ring outlines in dark mode. |
| `--ht-sidebar` | Deep Midnight Slate | `#171717` | Dark mode sidebar background. |
| `--ht-sidebar-fg` | Bright Silver-Gray | `#e5e5e5` | Sidebar option text in dark mode. |
| `--ht-sidebar-border`| Dark Sidebar Border | `rgba(255, 255, 255, 0.08)` | Sidebar separation borders. |
| `--ht-sidebar-accent`| Dark Sidebar Hover | `rgba(255, 255, 255, 0.10)` | Sidebar hover backgrounds. |

### Color Restraint & Status Signalling

Grayscale styling dominates. Accent colors are introduced only for contextual semantics:

* **Success:** Green (`#00a240` Light / `#22c55e` Dark) - Used for successful steps, done states, and positive confirmations.
* **Warning:** Yellow-Amber (`#e0ac00` Light / `#eab308` Dark) - Used for warnings and pending/attention states.
* **Danger:** Red (`#e02e2a` Light / `#ef4444` Dark) - Used for destructive operations, alerts, and runtime errors.
* **Info:** Blue (`#0169cc` Light / Auto-derived Indigo) - Used for active inline highlights and system announcements.
* **Comet Glow Gradient:** Rotating conic-gradient containing Blue (`#3b82f6`) -> Indigo (`#818cf8`) -> Orange (`#f97316`) -> Yellow (`#fbbf24`) -> Transparent. Utilized exclusively as the task running border overlay.

---

## 3. Typography Rules

Houston utilizes a pure, native modern typography system. It leverages the client system's font stack for high performance and zero external download layout shifts.

* **Primary Font Stack:** `ui-sans-serif, -apple-system, system-ui, "Segoe UI", Helvetica, Arial, sans-serif`
* **Text Treatment:** Typography is displayed in standard **Sentence case** for titles and headers. Decorative uppercase labels, wide tracking (`tracking-wider`), and italic configurations are avoided to maintain clean elegance.

### Typography Hierarchy

| Style / Element | Size | Weight | Tailwind Utility | Functional Role |
| :--- | :--- | :--- | :--- | :--- |
| **Main Header (h1)** | `28px` | `400` (Regular) | `text-[28px]` | Main screen titles and main workspace headers. |
| **Model Selector** | `18px` | `400` (Regular) | `text-lg` | Dropdowns and navigation titles. |
| **Body & Chat Input**| `16px` | `400` (Regular) | `text-base` | Text message feeds, descriptions, and user inputs. |
| **Buttons & Action text**| `14px` | `500` (Medium) | `text-sm font-medium` | Button labels, chip actions, and main menu labels. |
| **Sidebar Menu Items**| `14px` | `400` (Regular) | `text-sm` | Sidebar navigation rows and workspace items. |
| **Metadata Labels** | `12px` | `400` (Regular) | `text-xs` | Progress step indicators, small tags, and timestamps. |

---

## 4. Component Stylings

All UI components are styled using tailwind classes built on semantic vars.

### Buttons
All buttons use circular **Pill-shapes** (`rounded-full`) to differentiate them from square-corner content cards.

* **Primary CTA:** Solid near-black capsule.
  ```html
  class="bg-gray-950 text-white rounded-full h-9 px-3 text-sm font-medium hover:bg-gray-800 transition"
  ```
* **Secondary CTA:** White background, thin border outline.
  ```html
  class="bg-white text-gray-950 rounded-full h-9 px-3 border border-black/15 hover:bg-gray-50 transition"
  ```
* **Soft Chip:** Muted background pill.
  ```html
  class="bg-gray-100 rounded-full h-9 px-3 hover:bg-gray-200 transition"
  ```
* **Ghost Icon Button:** Large squared-off click target (icon centered).
  ```html
  class="bg-transparent rounded-lg w-9 h-9 flex items-center justify-center hover:bg-[#f3f3f3] transition"
  ```
* **Large Buttons:** Height grows to `h-11`, horizontal padding to `px-4`.

### Composer Input
The user workspace's signature input element. A single unified entry field with high-depth shadowing:
* **Shape:** Generously curved edges (`rounded-[28px]`), pure white canvas.
* **Padding & Layout:** `p-2.5 max-w-3xl`. Flex layout supporting trailing "Attach" controls, central "Textarea" inputs, and leading "Send" CTA.
* **Depth Shadows:** High-contrast layered shadows to anchor user attention:
  ```css
  box-shadow: 
    0 4px 4px rgba(0,0,0,0.04),
    0 4px 80px 8px rgba(0,0,0,0.04),
    0 0 1px rgba(0,0,0,0.62);
  ```

### Chat Messages
* **User Messages:** Right-aligned, encapsulated in soft pill bubbles (`rounded-3xl` with custom corners: `rounded-tr-sm`). White-gray canvas (`bg-[#f4f4f4]`) with generous padding (`px-5 py-2.5`), max-width 70%.
* **Assistant Messages:** Left-aligned, completely transparent background. Text is presented as plain markdown inside a standard grid with no enclosing bubble.
* **Inline Assistant Links:** Stylized as mini primary-pill buttons automatically:
  ```css
  .group.is-assistant a[href] {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 28px;
    padding: 0 12px;
    border-radius: 9999px;
    background: var(--color-primary);
    color: var(--color-primary-foreground);
    font-size: 12px;
    font-weight: 500;
    text-decoration: none;
    cursor: pointer;
    transition: opacity 0.2s;
  }
  ```
  Includes a dynamic mask-based external link indicator icon appended to the href anchor.

### Cards & Panels
* **Container Cards:** Rounded boundaries (`rounded-xl` / `rounded-2xl`), pure background canvas, thin borders (`border-black/5` or `--ht-border`). Subtle 1px edge shadow `0 1px 0 rgba(0,0,0,0.05)`. Hover raises the card with high diffusion.
* **Task Running Cards:** Card borders morph into rotating conic gradients when active using the `.card-running-glow` class.

### Form Inputs & Selectors
* **Stroke Outline:** Balanced input outlines (`border-gray-300` / `--ht-input`), rounded edges (`rounded-md`), flat background canvas.
* **Focus State:** Active selection replaces the outline with the thick primary ring (`--ht-ring`).

### Empty States
Consolidated via `Empty` from `@houston-ai/core`.
* **Aesthetic:** High-contrast `text-2xl font-semibold` titles, descriptive support text, single centered CTA. No large illustrative icon boxes to maintain minimalism.
* **Layout:** Centered column structure (`flex flex-col flex-1 justify-center`).

### Progress Panel
Right-aligned dynamic component (`ProgressPanel` from `@houston-ai/chat`).
* **Header:** Simple step tracker (`"X of Y steps complete"`).
* **States:** 
  - *Pending:* Muted empty circles.
  - *Active:* Animated circular loader spinner + high contrast labels.
  - *Done:* Bright green checkmark icon (`--ht-success`).

---

## 5. Layout Principles

Houston is built around a clear, split-pane horizontal composition structure.

```
+--------------------+-------------------------+----------------------+
| Sidebar (200px)    | Tab Bar / Header (52px) | Right Panel (45%)    |
| #f5f5f5            |-------------------------| (optional, resizable)|
|                    | Main Chat (max 768px)   |                      |
|                    |                         |                      |
+--------------------+-------------------------+----------------------+
```

* **Grid Boundaries:**
  - **Sidebar:** Width fixed at `200px`, background is a solid soft warm gray canvas (`#f5f5f5`). Used for agent lists and quick settings.
  - **Main workspace:** Dynamic layout, containing a top navigation header (`52px`) and main chat feed restricted to a max width of `768px` (`max-w-3xl`) to ensure optimal reading length.
  - **Right Sidebar panel:** Flex layout occupying up to 45% screen width (minimum `380px`). Integrates step lists, progress, and sidecar terminals.

* **Radii Constraints:**
  - **Chips / Tags:** `rounded` (0.25rem).
  - **Form Inputs:** `rounded-md`.
  - **Icon Buttons & Sidebar options:** `rounded-lg`.
  - **Standard Cards:** `rounded-xl`.
  - **Large Cards & Modals:** `rounded-2xl`.
  - **Composer Field:** `rounded-[28px]`.
  - **CTA buttons & Avatars:** `rounded-full`.

---

## 6. Interaction & Motion Rules

Visual animations in Houston serve functional purposes to convey system reactivity and status changes.

### Custom Motion Classes

1. **Card Running Glow (`card-running-glow`):**
   Rotating conic gradient comet-tail indicator for active workspace tasks. Spins infinitely over 2.5s using a conic-gradient mask:
   ```css
   .card-running-glow {
     --glow-angle: 0deg;
     background:
       linear-gradient(var(--glow-bg), var(--glow-bg)) padding-box,
       conic-gradient(
         from var(--glow-angle),
         transparent 60%,
         rgba(59, 130, 246, 0.15) 68%,
         #3b82f6 74%,
         #818cf8 78%,
         #f97316 82%,
         #fbbf24 84%,
         transparent 88%
       ) border-box;
     border: 1.5px solid transparent;
     animation: glow-spin 2.5s linear infinite;
   }
   ```
2. **Typing Loader Bounces (`typing-dot`):**
   Three bouncing dots staggered sequentially for background work statuses.
   ```css
   .typing-dot {
     animation: typing-bounce 1.2s ease-in-out infinite both;
   }
   .typing-dot:nth-child(1) { animation-delay: 0s; }
   .typing-dot:nth-child(2) { animation-delay: 0.15s; }
   .typing-dot:nth-child(3) { animation-delay: 0.3s; }
   ```
3. **Tool Activation Pulse (`tool-active-dot`):**
   Soft pulsing dot indicating an active tool execution block (1s duration transition).

### Motion Durations & Springs

* **Framer Motion Easing:** Custom cubic bezier curves preferred for entrance/exit states:
  - Entrance: `opacity: 0, y: 8` -> `opacity: 1, y: 0`.
  - Exit: `opacity: 0, y: -8`.
  - Bezier Curve: `[0.25, 0.1, 0.25, 1]` over `0.2s`.
* **Spring Animation Specs:** Dynamic physical spring settings are highly preferred for board layouts:
  ```json
  {
    "type": "spring",
    "stiffness": 300,
    "damping": 30,
    "mass": 1
  }
  ```

### Icon Rules
Houston leverages the **Lucide React** icon library exclusively. 
* **Sizes:** Standard icons are `20px` (`h-5 w-5`), small status indicators are `16px` (`h-4 w-4`), large screen headers are `24px` (`h-6 w-6`).
* **Stroke width:** Constant `2px` or `1.5px` (for lighter treatments). Icons always match `currentColor`.
* **Emojis:** Banned from layout icons. Avatars and user selections use standard Unicode symbols inside avatar containers when required, but static layout elements do not contain raw emoji structures.
