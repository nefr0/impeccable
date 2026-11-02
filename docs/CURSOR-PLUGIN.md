# Impeccable for Cursor

Design and refine interfaces with Impeccable: one text-only skill and specialist agents. Invoke `/impeccable polish`, `/impeccable audit`, or describe the design work you want.

## Installation

This native Cursor plugin is being prepared for marketplace review; it is not yet a published listing. For a project install, build the skill (`bun run build`) and copy `dist/cursor/.cursor/skills/impeccable` into your project's `.cursor/skills/`.

Use either that copy or the plugin, not both, to avoid duplicate skills. The plugin does not copy files into your project's `.cursor` directory. The plugin is text only: no scripts, no hooks, no downloads. Agent and shell permissions remain controlled by Cursor. Optional image generation requires its own provider credentials and approval.

## Local plugin preview

From the Impeccable repository, run `bun run build`, then copy `dist/cursor-plugin` into `~/.cursor/plugins/local/impeccable`. Do not overwrite an existing installation. Cursor 3.15.6 rejects symlinks whose targets are outside its local-plugin directory, so use a real copy. Reload Cursor, then inspect Customize for the skill and four agents. A marketplace installation with the same name takes precedence over a local preview. Local plugins are user-wide, even when testing with a separate editor profile; remove the temporary copy when finished.

Use a synthetic project outside the plugin directory to test context resolution. The documented `workspaceOpen`/`pluginPaths` alternative did not execute reliably during the 3.15.6 smoke test; it is not the verified preview path.

The plugin has no hook; it is text only.

## Verification status

Historical verification notes below predate the text-only fork (they mention a launcher and a pre-edit hook that no longer exist). Re-verify skill and agent discovery before relying on them.

On macOS with Cursor 3.15.6, the local plugin loaded with zero parser failures and exposed one skill plus four agents. Installing alongside a separate user skill produces duplicate slash-menu entries; choose one installation method.



## Maintainer submission checklist

- Public repository: https://github.com/pbakaus/impeccable
- Marketplace manifest: `.cursor-plugin/marketplace.json`; plugin source: `cursor-plugin/`.
- Native manifest, skill, agents, logo, and license are generated from source by `bun run build:release` and kept current by the generated-output workflow. Do not hand-edit `cursor-plugin/`.
- Verify `/impeccable` discovery, reference loading, and agent discovery and handoff paths in Cursor.
- After merge and verification, submit the repository at https://cursor.com/marketplace/publish. Submission and publication require maintainer approval. Replace this preparation notice and add the actual listing link only after acceptance.

References: [Cursor plugins](https://cursor.com/docs/plugins), [plugin reference](https://cursor.com/docs/reference/plugins), [hooks](https://cursor.com/docs/hooks).
