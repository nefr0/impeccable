# Impeccable

> **Fork differences.** This fork is text only.
> - The skill has no engine, no scripts and no hooks. The agent reads `PRODUCT.md`, `DESIGN.md`, surface briefs and platform references itself and applies the guidance.
> - No binary or file downloads, and no network requests from the skill.
> - Removed in this fork: font family, typeface and Google Fonts guidance (font size, scale and hierarchy stay).
> - The engine (`crates/`) is an optional standalone CLI, built locally with `cargo build --release -p impeccable`, never downloaded. The skill does not call it.
> - 22 commands (`live` and `generate` are removed).

The design vocabulary for your coding agent. Steer with `polish` and `bolder`, review with `critique` and `audit`, and avoid AI slop.

Build the skill and copy it into your tool:

```bash
bun run build
cp -r dist/claude-code/.claude/skills/impeccable your-project/.claude/skills/
```

Reload your coding tool, then run `/impeccable init` in your agent (`$impeccable init` on Codex).

Works with Claude Code, Cursor, Codex CLI, GitHub Copilot, Devin, Replit Agent, Gemini CLI, Grok Build, OpenCode, Google Antigravity, Kiro, Trae, Rovo Dev, Qoder, Mistral Vibe, Pi, Hermes Agent, DeepSeek Harness, and Veto. 

| Without Impeccable | With Impeccable |
|---|---|
| ![Villa Castagneto built without Impeccable: cream background, a thin serif headline with an italic accent, two pill buttons, and flat vector illustrations in rounded cards](https://github.com/pbakaus/impeccable/raw/main/docs/images/readme-without.webp) | ![Villa Castagneto built with Impeccable: a full-bleed engraved night scene of the villa on its cliff above the sea, a condensed cream and vermilion headline, and a month strip along the bottom](https://github.com/pbakaus/impeccable/raw/main/docs/images/readme-with.webp) |

Same model (Fable 5.1), same brief: a fourteen-room family guesthouse on the Ligurian coast. Without Impeccable, it builds the tasteful default: cream page, thin serif, italic accent, pill buttons. With it, the page is built around one picture: the house at night, seen from the water.

For a full project, see the [Neo Mirai case study](https://impeccable.style/cases/neo-mirai): a conference site taken from mockup to shipped page.

## Why

Purple-to-blue gradients. Cards nested in cards, gray text on colored backgrounds, a rounded-square icon tile above every heading. Ask an agent for a page with no other guidance and these show up again and again, across models and projects.

- **Words for what you want.** "Make it pop" leaves your agent to guess. `/impeccable bolder the pricing cards` loads a playbook written for that one move and points it at one target.
- **A reviewer.** `critique` scores the design against usability heuristics and ranks what to fix first. `audit` checks accessibility, performance, theming, and responsive behavior, with findings ranked P0 (blocking) to P3 (polish).

Impeccable is strongest on interfaces that already exist: point it at a page or a component and iterate. It builds new pages too; see [Build something new](https://impeccable.style/docs/new-work) for how to get the most out of that harder mode. To start, pick one component:

```
/impeccable critique the pricing cards
```

## How it works

Run `/impeccable init` once per project. It reads your code, asks only what it can't infer (who the product is for, what it needs to do, the constraints), and writes `PRODUCT.md`. Every later command reads it. Run `/impeccable document` to capture your existing visual system in `DESIGN.md`, so changes stay on-brand. [More on context files](https://impeccable.style/docs/context).

Then name a command and a target:

```
/impeccable critique the onboarding flow
/impeccable quieter the landing page
/impeccable polish settings
/impeccable harden checkout
```

Or just describe what you want: `/impeccable redo this hero section`. Narrow targets get better results; a single section reviewed closely beats a whole page at a glance.

### 22 commands

| | Commands |
|---|---|
| **Steer** | `bolder` amplifies safe designs · `quieter` tones down loud ones · `distill` removes what the page doesn't need · `colorize` brings color to a gray interface · `typeset` fixes hierarchy and sizing · `layout` fixes spacing and rhythm · `animate` adds transitions and interaction feedback · `delight` adds personality · `overdrive` goes for shaders, spring physics, and scroll-driven effects |
| **Review** | `critique` is a scored UX review · `audit` is a technical quality report |
| **Finish** | `polish` is the final pass before shipping · `harden` covers errors, i18n, overflow, and edge cases · `clarify` rewrites unclear copy · `adapt` handles screen sizes and devices · `optimize` fixes loading and rendering performance · `onboard` designs first-run flows and empty states |
| **Plan and set up** | `shape` plans UX before code · `init` writes `PRODUCT.md` · `document` writes `DESIGN.md` · `extract` pulls repeated patterns into your design system |

The 22nd, `craft`, is a deprecated alias for a plain build request. Type `/impeccable` alone to see the list in your agent. Each command has its own page: [impeccable.style/docs](https://impeccable.style/docs).

On Codex, open `/skills` or type `$impeccable` instead of the slash command.

## Standalone engine (optional)

The Rust engine in `crates/` is a standalone CLI that detects UI anti-patterns in files and rendered pages. The skill does not call it. Build it locally:

```bash
cargo build --release -p impeccable
target/release/impeccable detect src/
```

See [docs/ENGINE.md](docs/ENGINE.md) and [docs/CLI-CONTRACT.md](docs/CLI-CONTRACT.md). Engine-only features (live mode, design hook, detector, `doctor`, telemetry, font matching) were removed from the skill in this fork.

## Installation

```bash
bun run build
```

This writes one folder per provider under `dist/`. Copy `skills/impeccable` from the matching folder into your project or your global skills folder.

| Tool | Build output | Project folder | Global skills folder |
|---|---|---|---|
| Claude Code | `dist/claude-code/.claude/` | `.claude/skills/` | `~/.claude/skills/` |
| Cursor | `dist/cursor/.cursor/` | `.cursor/skills/` | |
| Codex CLI, Replit Agent | `dist/agents/.agents/` | `.agents/skills/` | `~/.agents/skills/` |
| Gemini CLI | `dist/gemini/.gemini/` | `.gemini/skills/` | |
| GitHub Copilot | `dist/github/.github/` | `.github/skills/` | |
| OpenCode | `dist/opencode/.opencode/` | `.opencode/skills/` | |
| Grok Build | `dist/grok/.grok/` | `.grok/skills/` | `~/.grok/skills/` |
| Other tools | `dist/<provider>/` | see `docs/HARNESSES.md` | |

Check `dist/` for the exact provider folder names. For example: `mkdir -p your-project/.claude/skills && cp -r dist/claude-code/.claude/skills/impeccable your-project/.claude/skills/`.

Notes:
- OpenCode also needs the generated `commands/impeccable.md` copied into your project's `.opencode/commands/` to provide `/impeccable`.
- Cursor and Gemini CLI may need Agent Skills enabled in settings. See the [Cursor](https://cursor.com/docs/context/skills) and [Gemini CLI](https://geminicli.com/docs/cli/skills/) skills docs.
- Restart Trae after copying.
- Per-tool details live in [docs/HARNESSES.md](docs/HARNESSES.md).

## Project files

The skill reads `PRODUCT.md` and `DESIGN.md` from your repo, plus optional surface briefs under `.impeccable/surfaces/*.md`. Keep them in the repo. The skill runs no code and writes no runtime state.

## Community

- [Issues](https://github.com/pbakaus/impeccable/issues) for bugs, questions, and ideas. See [DEVELOP.md](docs/DEVELOP.md).

## License

Apache 2.0. See [LICENSE](LICENSE). Impeccable started from Anthropic's [frontend-design](https://github.com/anthropics/skills/tree/main/skills/frontend-design) skill.

---

Created by [Paul Bakaus](https://www.paulbakaus.com)
