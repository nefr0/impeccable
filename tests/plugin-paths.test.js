/**
 * Unit coverage for the plugin subtree markdown handling (issue #736).
 *
 * The ./plugin subtree copies the dist/claude-code output. The skill is text
 * only (no scripts), so the only rewrite is dropping `allowed-tools` from the
 * SKILL.md frontmatter; every other byte passes through untouched.
 */
import { describe, test, expect, beforeEach, afterEach } from 'bun:test';
import fs from 'fs';
import os from 'os';
import path from 'path';
import {
  rewritePluginMarkdown,
  rewritePluginAgentMarkdown,
  rewritePluginMarkdownTree,
  verifyPluginSkillRewrite,
  verifyPluginAgentRewrite,
} from '../scripts/lib/plugin-paths.js';

describe('rewritePluginMarkdown', () => {
  test('removes the entire allowed-tools frontmatter block', () => {
    const input = [
      '---',
      'name: impeccable',
      'allowed-tools:',
      '  - Read',
      '  - Write',
      'license: Apache 2.0',
      '---',
      '',
      'Body text.',
    ].join('\n');
    const output = rewritePluginMarkdown(input);
    expect(output).not.toMatch(/^allowed-tools:/m);
    expect(output).not.toContain('- Read');
    expect(output).toContain('license: Apache 2.0');
    expect(output).toContain('Body text.');
  });

  test('returns content without allowed-tools byte-for-byte', () => {
    const input = '---\nname: impeccable\n---\n\nBody mentions .claude/skills/impeccable/reference/x.md.\n';
    expect(rewritePluginMarkdown(input)).toBe(input);
  });

  test('leaves body paths alone when it strips allowed-tools', () => {
    const input = '---\nname: x\nallowed-tools: Read\n---\n\nSee .claude/skills/impeccable/reference/x.md.\n';
    expect(rewritePluginMarkdown(input)).toContain('See .claude/skills/impeccable/reference/x.md.');
  });
});

describe('rewritePluginAgentMarkdown', () => {
  test('is an identity transform', () => {
    const input = '---\nname: agent\n---\n\nAgent body.\n';
    expect(rewritePluginAgentMarkdown(input)).toBe(input);
  });
});

describe('verifyPluginAgentRewrite', () => {
  let root;

  beforeEach(() => {
    root = fs.mkdtempSync(path.join(os.tmpdir(), 'impeccable-agent-verify-'));
  });

  afterEach(() => {
    fs.rmSync(root, { recursive: true, force: true });
  });

  test('accepts an existing agent file', () => {
    const agentPath = path.join(root, 'agent.md');
    fs.writeFileSync(agentPath, 'Agent body.\n');
    expect(() => verifyPluginAgentRewrite(agentPath)).not.toThrow();
  });

  test('fails when the agent file is missing', () => {
    expect(() => verifyPluginAgentRewrite(path.join(root, 'missing.md'))).toThrow(/Plugin agent missing/);
  });
});

describe('rewritePluginMarkdownTree', () => {
  let root;

  beforeEach(() => {
    root = fs.mkdtempSync(path.join(os.tmpdir(), 'impeccable-plugin-paths-'));
  });

  afterEach(() => {
    fs.rmSync(root, { recursive: true, force: true });
  });

  test('rewrites .md files recursively and leaves other files untouched', () => {
    fs.mkdirSync(path.join(root, 'reference'), { recursive: true });
    const skillMd = path.join(root, 'SKILL.md');
    const refMd = path.join(root, 'reference', 'x.md');
    const other = path.join(root, 'notes.txt');
    fs.writeFileSync(skillMd, '---\nname: x\nallowed-tools: Read\n---\n\nBody.\n');
    fs.writeFileSync(refMd, '---\nname: r\nallowed-tools: Read\n---\n\nRef.\n');
    fs.writeFileSync(other, 'allowed-tools: Read\n');

    rewritePluginMarkdownTree(root);

    expect(fs.readFileSync(skillMd, 'utf-8')).not.toContain('allowed-tools');
    expect(fs.readFileSync(refMd, 'utf-8')).not.toContain('allowed-tools');
    expect(fs.readFileSync(other, 'utf-8')).toBe('allowed-tools: Read\n');
  });

  test('applies a custom rewrite when passed', () => {
    const agentMd = path.join(root, 'agent.md');
    fs.writeFileSync(agentMd, 'Agent body.\n');
    rewritePluginMarkdownTree(root, rewritePluginAgentMarkdown);
    expect(fs.readFileSync(agentMd, 'utf-8')).toBe('Agent body.\n');
  });

  test('is a no-op on a missing directory', () => {
    expect(() => rewritePluginMarkdownTree(path.join(root, 'does-not-exist'))).not.toThrow();
  });
});

describe('verifyPluginSkillRewrite', () => {
  let root;

  beforeEach(() => {
    root = fs.mkdtempSync(path.join(os.tmpdir(), 'impeccable-plugin-verify-'));
  });

  afterEach(() => {
    fs.rmSync(root, { recursive: true, force: true });
  });

  const writeSkill = (contents) => {
    const skillPath = path.join(root, 'SKILL.md');
    fs.writeFileSync(skillPath, contents);
    return skillPath;
  };

  test('accepts a SKILL.md with allowed-tools removed', () => {
    const skillPath = writeSkill(rewritePluginMarkdown('---\nname: x\nallowed-tools: Read\n---\n\nBody.\n'));
    expect(() => verifyPluginSkillRewrite(skillPath)).not.toThrow();
  });

  test('fails the build when allowed-tools survives', () => {
    const skillPath = writeSkill('---\nname: x\nallowed-tools: Read\n---\n\nBody.\n');
    expect(() => verifyPluginSkillRewrite(skillPath)).toThrow(/still declares allowed-tools/);
  });
});
