# Impeccable CLI

> Standalone CLI; the skill does not call it. This fork publishes nothing to npm and downloads no binaries.

Detect UI anti-patterns and design quality issues from the command line. The detector scans HTML, CSS, JSX, TSX, Vue, and Svelte files for 62 deterministic rules, including AI-generated UI tells, accessibility violations, and general design quality problems.

The `cli/` shim runs the `impeccable` engine binary that you build locally.

## Quick Start

```bash
cargo build --release -p impeccable
export IMPECCABLE_BIN="$PWD/target/release/impeccable"

# Scan files or directories for anti-patterns
node cli/bin/cli.js detect src/

# Scan a live URL (uses an installed Chrome, Chromium, or Edge)
node cli/bin/cli.js detect https://example.com

# JSON output for CI/tooling
node cli/bin/cli.js detect --json src/

# GitHub Actions annotations
node cli/bin/cli.js detect --github src/

# Catalog of detector rules (no scan)
node cli/bin/cli.js detect --list-rules
```

To install the skill itself, build it with `bun run build` and copy it from `dist/` (see README.md).

## What It Detects

**AI Slop Tells**: patterns that scream "AI generated this":
- Side-tab accent borders, gradient text on headings
- Purple/violet gradients and cyan-on-dark palettes
- Dark mode with glowing accents, border + border-radius clashes

**Typography Issues**: flat type hierarchy, tiny text

**Color & Contrast**: WCAG AA violations, gray text on colored backgrounds, pure black/white

**Layout & Composition**: nested cards, monotonous spacing, everything-centered layouts

**Motion**: bounce/elastic easing, layout property transitions

**Quality**: tiny body text, cramped padding, long line lengths, small touch targets

62 deterministic detector rules in total. See the full catalog at [impeccable.style/slop](https://impeccable.style/slop).

## Exit Codes

- `0`: scan completed with no primary findings (advisories may still be listed)
- `1`: at least one requested target could not be scanned
- `2`: scan completed with primary findings

Operational failure takes precedence when a multi-target scan is partial. In JSON mode, stdout remains a findings array and diagnostics are written to stderr. With `--github`, stdout is one GitHub Actions annotation per finding; GitHub displays at most 10 annotations of each type per step and 50 per job, and the rest stay in the log.

## Options

```
impeccable detect [options] [file-or-dir-or-url...]

  --json        Output findings as JSON
  --github      Write GitHub Actions annotations for each finding
  --list-rules  Print the detector rule catalog and exit
  --scope       Only report rules in a design domain (type, layout)
  --help        Show help
```

## Requirements

- Node.js 18.19+ to run the shim. The engine itself is a self-contained binary built locally.
- For URL scans, an installed Chrome, Chromium, or Edge (set `IMPECCABLE_BROWSER` to point at one).
- Set `IMPECCABLE_BIN` to your local build. The shim never downloads a binary.

## Part of Impeccable

The skill is a text-only design pack with 22 commands. This CLI is an optional companion.

## License

[Apache 2.0](https://github.com/pbakaus/impeccable/blob/main/LICENSE)
