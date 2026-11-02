import { afterEach, beforeEach, describe, it } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { stageCursorPlugin, rewriteCursorPluginMarkdown } from '../scripts/lib/cursor-plugin.js';
import { collectPluginVersions } from '../scripts/lib/validate-plugin-versions.js';
import { readSourceFiles } from '../scripts/lib/utils.js';
import { createTransformer, PROVIDERS } from '../scripts/lib/transformers/index.js';

const sample = `---
name: impeccable
description: Design
---
Read PRODUCT.md and DESIGN.md, then apply the guidance.   
`;

describe('Cursor native plugin', () => {
  let root;
  const write = (rel, content) => {
    const file = path.join(root, rel);
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, content);
  };
  beforeEach(() => {
    root = fs.mkdtempSync(path.join(os.tmpdir(), 'impeccable-cursor-plugin-'));
    write('.claude-plugin/plugin.json', JSON.stringify({ version: '4.2.2', homepage: 'https://impeccable.style', repository: 'https://github.com/pbakaus/impeccable' }));
    write('scripts/lib/assets/plugin-icon.png', 'icon');
    write('docs/CURSOR-PLUGIN.md', 'Readme');
    write('LICENSE', 'license');
    write('dist/cursor/.cursor/skills/impeccable/SKILL.md', sample);
    write('dist/cursor/.cursor/skills/impeccable/reference/polish.md', sample);
    write('dist/cursor/.cursor/agents/impeccable-asset-producer.md', sample);
  });
  afterEach(() => fs.rmSync(root, { recursive: true, force: true }));

  it('stages only the Cursor payload, with no hooks and no scripts', () => {
    const output = stageCursorPlugin(root, path.join(root, 'dist'));
    const manifest = JSON.parse(fs.readFileSync(path.join(output, '.cursor-plugin/plugin.json')));
    assert.equal(manifest.version, '4.2.2');
    assert.equal(manifest.skills, './skills/');
    assert.equal(manifest.agents, './agents/');
    assert.equal('hooks' in manifest, false);
    for (const rel of [manifest.logo, 'README.md', 'LICENSE']) assert.ok(fs.existsSync(path.join(output, rel)));
    assert.equal(fs.existsSync(path.join(output, 'hooks')), false);
    assert.equal(fs.existsSync(path.join(output, 'skills/impeccable/scripts')), false);
    for (const rel of ['skills/impeccable/SKILL.md', 'skills/impeccable/reference/polish.md', 'agents/impeccable-asset-producer.md']) {
      const text = fs.readFileSync(path.join(output, rel), 'utf8');
      assert.match(text, /^---\nname: /);
      assert.doesNotMatch(text, /[\t ]+$/m);
      assert.ok(text.endsWith('apply the guidance.\n'));
    }
    assert.equal(fs.readFileSync(path.join(root, 'dist/cursor/.cursor/skills/impeccable/SKILL.md'), 'utf8'), sample);
    write('dist/cursor-plugin/stale.txt', 'stale');
    stageCursorPlugin(root, path.join(root, 'dist'));
    assert.equal(fs.existsSync(path.join(output, 'stale.txt')), false);
  });

  it('does not add path prose to agents that do not use scripts', () => {
    assert.equal(rewriteCursorPluginMarkdown('---\nname: reviewer\n---\nReview.', { agent: true }), '---\nname: reviewer\n---\nReview.\n');
  });

  it('detects version drift in both staged and tracked Cursor artifacts', () => {
    for (const prefix of ['cursor-plugin', 'dist/cursor-plugin']) {
      write(`${prefix}/.cursor-plugin/plugin.json`, JSON.stringify({ version: '0.0.1' }));
      write(`${prefix}/skills/impeccable/SKILL.md`, '---\nversion: 0.0.1\n---\n');
    }
    assert.equal(collectPluginVersions(root).mismatches.length, 4);
  });

  it('keeps the Git marketplace source connected to the staged plugin', () => {
    const repository = path.resolve(import.meta.dirname, '..');
    const marketplace = JSON.parse(fs.readFileSync(path.join(repository, '.cursor-plugin/marketplace.json')));
    assert.equal(marketplace.plugins[0].source, './cursor-plugin');
  });

  it('packages the real provider skill and all agents as text only', () => {
    const repository = path.resolve(import.meta.dirname, '..');
    const version = JSON.parse(fs.readFileSync(path.join(repository, '.claude-plugin/plugin.json'))).version;
    const { skills } = readSourceFiles(repository);
    const dist = path.join(root, 'real-provider');
    createTransformer(PROVIDERS.cursor)(skills, dist, { skillsVersion: version });
    const output = stageCursorPlugin(repository, dist);
    assert.equal(fs.readdirSync(path.join(output, 'agents')).length, skills[0].agents.length);
    let links = 0;
    for (const rel of fs.readdirSync(output, { recursive: true }).filter(file => file.endsWith('.md'))) {
      const filename = path.join(output, rel);
      const text = fs.readFileSync(filename, 'utf8');
      assert.doesNotMatch(text, /\{\{scripts_path\}\}|<skill-base-dir>/);
      for (const match of text.matchAll(/\]\(([^)\s]+)\)/g)) {
        if (!/^reference\//.test(match[1])) continue;
        assert.ok(fs.existsSync(path.resolve(path.dirname(filename), match[1].split('#')[0])), `${rel}: ${match[1]}`);
        links++;
      }
    }
    assert.ok(links > 0);
    assert.equal(fs.existsSync(path.join(output, 'skills/impeccable/scripts')), false);
    assert.equal(fs.existsSync(path.join(output, 'hooks')), false);
  });
});
