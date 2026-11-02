import fs from 'fs';
import path from 'path';
import { generateYamlFrontmatter, parseFrontmatter } from './utils.js';

/**
 * Markdown handling for the ./plugin subtree, a copy of the dist/claude-code
 * output. The skill is text only: it ships no scripts, so no path rewriting
 * is needed. The only transformation is dropping `allowed-tools` from the
 * SKILL.md frontmatter.
 *
 * Claude Code requires user consent to activate a skill whose frontmatter
 * declares allowed-tools; non-interactive hosts (`claude -p`) cannot provide
 * it and the skill silently degrades (issue #736). The plugin copy drops the
 * key via structured frontmatter rewrite, not regex.
 */
function stripAllowedToolsFrontmatter(content) {
  const { frontmatter, body } = parseFrontmatter(content);
  if (frontmatter['allowed-tools'] === undefined) return content;
  delete frontmatter['allowed-tools'];
  return `${generateYamlFrontmatter(frontmatter)}\n${body}`;
}

/** Rewrite one markdown file's content for the plugin subtree. Pure. */
export function rewritePluginMarkdown(content) {
  return stripAllowedToolsFrontmatter(content);
}

/** Agent files need no rewrite; kept so the tree walker has one shape. */
export function rewritePluginAgentMarkdown(content) {
  return content;
}

/** Agent files carry no plugin-specific content to verify. */
export function verifyPluginAgentRewrite(agentPath) {
  if (!fs.existsSync(agentPath)) throw new Error(`Plugin agent missing: ${agentPath}`);
}

/**
 * Fail the build when the copied SKILL.md still declares allowed-tools,
 * which would block activation in non-interactive sessions (issue #736).
 */
export function verifyPluginSkillRewrite(skillMdPath) {
  const content = fs.readFileSync(skillMdPath, 'utf-8');
  if (parseFrontmatter(content).frontmatter['allowed-tools'] !== undefined) {
    throw new Error(
      `Plugin rewrite drift: ${skillMdPath} still declares allowed-tools in frontmatter. ` +
      'The plugin copy must drop the entire block so non-interactive sessions can activate ' +
      'the skill (issue #736); update the removal in scripts/lib/plugin-paths.js.',
    );
  }
}

/**
 * Apply a rewrite to every .md file under dir, recursively. Defaults to
 * the skill rewrite; the agents directory passes rewritePluginAgentMarkdown.
 */
export function rewritePluginMarkdownTree(dir, rewrite = rewritePluginMarkdown) {
  if (!fs.existsSync(dir)) return;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const entryPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      rewritePluginMarkdownTree(entryPath, rewrite);
    } else if (entry.name.endsWith('.md')) {
      const original = fs.readFileSync(entryPath, 'utf-8');
      const rewritten = rewrite(original, { isSkillEntrypoint: entry.name === 'SKILL.md' });
      if (rewritten !== original) fs.writeFileSync(entryPath, rewritten);
    }
  }
}
