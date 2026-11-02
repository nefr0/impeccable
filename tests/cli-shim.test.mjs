/**
 * The npm shim (`cli/bin/cli.js`) resolves a local engine binary and never
 * downloads one. Order: IMPECCABLE_BIN, skill/scripts/bin/<os>-<arch>/ beside
 * the shim, ~/.impeccable/bin/impeccable, then `impeccable-engine` on PATH.
 * Each case runs a staged copy of the shim so the sibling lookup is isolated.
 */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it } from 'node:test';

const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const PKG_PATH = path.join(REPO, 'package.json');
const TARGET = `${{ darwin: 'darwin', linux: 'linux', win32: 'windows' }[process.platform] || process.platform}`
  + `-${{ arm64: 'arm64', x64: 'x64' }[process.arch] || process.arch}`;

function fakeEngine(file, label) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, `#!/bin/sh\necho "${label} $*"\n`, { mode: 0o755 });
}

function stageShim() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'impeccable-shim-'));
  fs.mkdirSync(path.join(dir, 'cli', 'bin'), { recursive: true });
  fs.copyFileSync(PKG_PATH, path.join(dir, 'package.json'));
  fs.copyFileSync(path.join(REPO, 'cli', 'bin', 'cli.js'), path.join(dir, 'cli', 'bin', 'cli.js'));
  return { dir, shim: path.join(dir, 'cli', 'bin', 'cli.js'), home: path.join(dir, 'home'), pathDir: path.join(dir, 'pathbin') };
}

function run(stage, args, extraEnv = {}) {
  const env = { ...process.env, HOME: stage.home, USERPROFILE: stage.home, PATH: stage.pathDir, ...extraEnv };
  if (!('IMPECCABLE_BIN' in extraEnv)) delete env.IMPECCABLE_BIN;
  const res = spawnSync(process.execPath, [stage.shim, ...args], { env, encoding: 'utf8' });
  return { status: res.status, stdout: res.stdout, stderr: res.stderr };
}

describe('npm shim local engine lookup', { skip: process.platform === 'win32' ? 'posix only' : false }, () => {
  it('prefers IMPECCABLE_BIN over every other candidate', () => {
    const stage = stageShim();
    fakeEngine(path.join(stage.dir, 'skill', 'scripts', 'bin', TARGET, 'impeccable'), 'sibling');
    fakeEngine(path.join(stage.dir, 'envbin'), 'env-bin');
    const res = run(stage, ['hi'], { IMPECCABLE_BIN: path.join(stage.dir, 'envbin') });
    assert.equal(res.status, 0, res.stderr);
    assert.match(res.stdout, /env-bin hi/);
  });

  it('uses the sibling skill/scripts/bin binary before the home binary', () => {
    const stage = stageShim();
    fakeEngine(path.join(stage.dir, 'skill', 'scripts', 'bin', TARGET, 'impeccable'), 'sibling');
    fakeEngine(path.join(stage.home, '.impeccable', 'bin', 'impeccable'), 'home');
    const res = run(stage, ['hi']);
    assert.equal(res.status, 0, res.stderr);
    assert.match(res.stdout, /sibling hi/);
  });

  it('uses ~/.impeccable/bin/impeccable before PATH', () => {
    const stage = stageShim();
    fakeEngine(path.join(stage.home, '.impeccable', 'bin', 'impeccable'), 'home');
    fakeEngine(path.join(stage.pathDir, 'impeccable-engine'), 'path-engine');
    const res = run(stage, ['hi']);
    assert.equal(res.status, 0, res.stderr);
    assert.match(res.stdout, /home hi/);
  });

  it('falls back to impeccable-engine on PATH', () => {
    const stage = stageShim();
    fakeEngine(path.join(stage.pathDir, 'impeccable-engine'), 'path-engine');
    const res = run(stage, ['hi']);
    assert.equal(res.status, 0, res.stderr);
    assert.match(res.stdout, /path-engine hi/);
  });

  it('exits 127 with build instructions when no engine is found', () => {
    const stage = stageShim();
    fs.mkdirSync(stage.pathDir, { recursive: true });
    const res = run(stage, ['hi']);
    assert.equal(res.status, 127);
    assert.match(res.stderr, /build it: cargo build --release -p impeccable, then set IMPECCABLE_BIN/);
  });

  it('answers --version and -v from its own package.json without a binary', () => {
    const expected = JSON.parse(fs.readFileSync(PKG_PATH, 'utf-8')).version;
    for (const args of [['--version'], ['-v'], ['--version', 'extra']]) {
      const res = run(stageShim(), args);
      assert.equal(res.status, 0, `${args.join(' ')} exits 0`);
      assert.equal(res.stdout, `${expected}\n`);
    }
  });

  it('contains no download or fetch code', () => {
    const source = fs.readFileSync(path.join(REPO, 'cli', 'bin', 'cli.js'), 'utf8');
    assert.doesNotMatch(source, /fetch\(|optionalDependencies|releases\/download/);
  });
});
