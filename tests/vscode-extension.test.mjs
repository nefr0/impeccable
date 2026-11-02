import { after, before, describe, it } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { readSourceFiles } from '../scripts/lib/utils.js';
import { createTransformer, PROVIDERS } from '../scripts/lib/transformers/index.js';
import { stageVSCodeExtension, rewriteVSCodeMarkdown } from '../scripts/lib/vscode-extension.js';

const repo = fileURLToPath(new URL('..', import.meta.url));

describe('VS Code skill extension', () => {
  let scratch;
  let extension;
  before(() => {
    scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'impeccable-vscode-test-'));
    const { skills } = readSourceFiles(repo);
    const skillsVersion = JSON.parse(fs.readFileSync(path.join(repo, '.claude-plugin/plugin.json'), 'utf8')).version;
    createTransformer(PROVIDERS.github)(skills, scratch, { skillsVersion });
    extension = stageVSCodeExtension(repo, scratch);
  });
  after(() => fs.rmSync(scratch, { recursive: true, force: true }));

  it('registers only the skill, without editor runtime or project-install sidecars', () => {
    const manifest = JSON.parse(fs.readFileSync(path.join(extension, 'package.json'), 'utf8'));
    assert.equal(manifest.publisher, 'renaissance-geek');
    assert.deepEqual(manifest.contributes, { chatSkills: [{ path: './skills/impeccable/SKILL.md' }] });
    for (const key of ['main', 'browser', 'activationEvents', 'scripts']) assert.equal(key in manifest, false);
    assert.equal(manifest.version, JSON.parse(fs.readFileSync(path.join(repo, '.claude-plugin/plugin.json'))).version);
    assert.deepEqual(fs.readdirSync(extension).sort(), ['.vscodeignore', 'LICENSE', 'README.md', 'icon.png', 'package.json', 'skills']);
    assert.ok(fs.existsSync(path.join(extension, manifest.contributes.chatSkills[0].path)));
    const skill = fs.readFileSync(path.join(extension, manifest.contributes.chatSkills[0].path), 'utf8');
    assert.equal(skill.match(/^version: (.+)$/m)?.[1], manifest.version);
    assert.ok(fs.existsSync(path.join(extension, 'skills/impeccable/reference/degraded/asset-producer.md')));
    assert.equal(fs.existsSync(path.join(extension, 'skills/impeccable/scripts')), false);
  });

  it('resolves linked resources inside the packaged skill and ships no script paths', () => {
    const skill = path.join(extension, 'skills/impeccable');
    let links = 0;
    for (const rel of fs.readdirSync(skill, { recursive: true }).filter(p => p.endsWith('.md'))) {
      const filename = path.join(skill, rel);
      const content = fs.readFileSync(filename, 'utf8');
      assert.doesNotMatch(content, /\{\{scripts_path\}\}|\$\{CLAUDE_(?:SKILL_DIR|PLUGIN_ROOT)\}|<skill-base-dir>/);
      for (const match of content.matchAll(/\]\(([^)\s]+)\)/g)) {
        const link = match[1];
        if (!/^reference\//.test(link)) continue;
        assert.ok(fs.existsSync(path.resolve(path.dirname(filename), link.split('#')[0])), `${rel}: ${link}`);
        links++;
      }
    }
    assert.ok(links > 0, 'entrypoint retains its playbook links');
  });

  it('ships markdown unchanged', () => {
    const input = 'Read PRODUCT.md and DESIGN.md.\n';
    assert.equal(rewriteVSCodeMarkdown(input), input);
  });

  it('rebuilds deterministically and removes stale generated files', () => {
    const before = fs.readFileSync(path.join(extension, 'skills/impeccable/SKILL.md'));
    fs.writeFileSync(path.join(extension, 'old.js'), 'stale');
    stageVSCodeExtension(repo, scratch);
    assert.equal(fs.existsSync(path.join(extension, 'old.js')), false);
    assert.deepEqual(fs.readFileSync(path.join(extension, 'skills/impeccable/SKILL.md')), before);
  });
});
