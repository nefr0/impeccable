#!/usr/bin/env node
// `impeccable` npm shim: finds a local engine binary and execs it with argv.
// Order: $IMPECCABLE_BIN, bin/<os>-<arch>/ next to the skill launcher
// (skill/scripts), ~/.impeccable/bin/impeccable, then `impeccable-engine` on
// PATH. Never downloads anything.
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import { createRequire } from 'node:module';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
const pkg = require('../../package.json');
const OS = { darwin: 'darwin', linux: 'linux', win32: 'windows' }[process.platform] || process.platform;
const ARCH = { arm64: 'arm64', x64: 'x64' }[process.arch] || process.arch;
const TARGET = `${OS}-${ARCH}`;
const EXE = OS === 'windows' ? 'impeccable.exe' : 'impeccable';
const ENGINE_NAME = OS === 'windows' ? 'impeccable-engine.exe' : 'impeccable-engine';
const SHIM_DIR = path.dirname(fileURLToPath(import.meta.url));
const SIBLING = path.join(SHIM_DIR, '..', '..', 'skill', 'scripts', 'bin', TARGET, EXE);
const HOME_BIN = path.join(os.homedir(), '.impeccable', 'bin', EXE);

function exists(p) { try { return !!p && fs.statSync(p).isFile(); } catch { return false; } }
function onPath(name) {
  for (const dir of (process.env.PATH || '').split(path.delimiter)) {
    if (dir && exists(path.join(dir, name))) return path.join(dir, name);
  }
  return null;
}
function locate() {
  const envBin = process.env.IMPECCABLE_BIN;
  if (exists(envBin)) return envBin;
  if (exists(SIBLING)) return SIBLING;
  if (exists(HOME_BIN)) return HOME_BIN;
  return onPath(ENGINE_NAME);
}

// `--version` / `-v` is answered by the shim itself: the number users mean
// is this npm package's version, not the engine's (docs/CLI-CONTRACT.md).
const argv = process.argv.slice(2);
if (argv[0] === '--version' || argv[0] === '-v') {
  process.stdout.write(`${pkg.version}\n`);
  process.exit(0);
}

const bin = locate();
if (!bin) {
  process.stderr.write(
    `impeccable: no engine binary for ${TARGET} (looked in IMPECCABLE_BIN, ${SIBLING}, ${HOME_BIN}, PATH).\n`
    + 'build it: cargo build --release -p impeccable, then set IMPECCABLE_BIN\n',
  );
  process.exit(127);
}
const result = spawnSync(bin, argv, {
  stdio: 'inherit',
  env: { IMPECCABLE_SELF: 'npx impeccable', ...process.env },
});
if (result.error) {
  process.stderr.write(`impeccable: failed to run ${bin}: ${result.error.message}\n`);
  process.exit(127);
}
process.exit(result.status === null ? 1 : result.status);
