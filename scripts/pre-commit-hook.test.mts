import { after, describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, delimiter, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

/**
 * Tests for the gitleaks pre-commit hook and its config/workflow wiring.
 * Fixtures live only in temporary repositories; the stubs stand in for gitleaks,
 * so no secret-shaped literal exists here.
 */
const REPO_ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const HOOK_REL = join('.githooks', 'pre-commit');
const HOOK = join(REPO_ROOT, HOOK_REL);
const CONFIG = join(REPO_ROOT, '.gitleaks.toml');
const WORKFLOW = join(REPO_ROOT, '.github', 'workflows', 'secret-scan.yml');
const read = (path: string): string => readFileSync(path, 'utf8').replace(/\r\n/g, '\n');

const findOnPath = (name: string): string | undefined =>
  (process.env['PATH'] ?? process.env['Path'] ?? '')
    .split(delimiter)
    .filter(Boolean)
    .map((folder) => resolve(folder, name))
    .find((file) => existsSync(file));
const GIT = findOnPath(process.platform === 'win32' ? 'git.exe' : 'git');
if (!GIT) throw new Error('Git executable unavailable');
// git runs hooks with its own sh; skip execution tests when no POSIX shell is reachable.
const SH =
  findOnPath(process.platform === 'win32' ? 'sh.exe' : 'sh') ??
  (process.platform === 'win32'
    ? [join(dirname(dirname(GIT)), 'usr', 'bin', 'sh.exe'), join(dirname(dirname(GIT)), 'bin', 'sh.exe')].find(existsSync)
    : undefined);
const SKIP_SH = SH ? false : 'sh is not available; hook execution tests skipped';

const tempDirs: string[] = [];
after(() => {
  for (const dir of tempDirs) {
    assert.equal(dirname(realpathSync(dir)), realpathSync(tmpdir()));
    assert.ok(basename(dir).startsWith('dd-hook-'));
    rmSync(dir, { recursive: true, force: true });
  }
});

function git(root: string, args: string[]): string {
  const result = spawnSync(GIT!, ['-C', root, '-c', 'commit.gpgsign=false', ...args], {
    encoding: 'utf8',
  });
  assert.equal(result.status, 0, result.stderr);
  return result.stdout.trim();
}
function writeFile(root: string, rel: string, content: string): void {
  const abs = join(root, rel);
  mkdirSync(dirname(abs), { recursive: true });
  writeFileSync(abs, content);
}
function newRepoWithHook(): string {
  const root = mkdtempSync(join(tmpdir(), 'dd-hook-'));
  tempDirs.push(root);
  git(root, ['init', '-q']);
  git(root, ['config', 'user.name', 'synthetic']);
  git(root, ['config', 'user.email', 'synthetic@example.invalid']);
  copyFileSync(CONFIG, join(root, '.gitleaks.toml'));
  mkdirSync(join(root, '.githooks'), { recursive: true });
  copyFileSync(HOOK, join(root, HOOK_REL));
  chmodSync(join(root, HOOK_REL), 0o755);
  git(root, ['config', 'core.hooksPath', '.githooks']);
  writeFile(root, '.gitignore', '.tools/\n');
  writeFile(root, 'src/app.ts', 'export const ok = 1;\n');
  git(root, ['add', '-A']);
  return root;
}
const commit = (root: string, env: NodeJS.ProcessEnv = process.env) =>
  spawnSync(
    GIT!,
    ['-C', root, '-c', 'commit.gpgsign=false', 'commit', '-q', '-m', 'hook fixture'],
    { encoding: 'utf8', env },
  );
const out = (r: { stdout: string; stderr: string }) => r.stdout + r.stderr;
const STUB_DIR = '.tools/gitleaks/8.30.1';

describe('static wiring', () => {
  const hook = read(HOOK);
  it('hook is a strict POSIX sh script pinned to 8.30.1 that scans staged changes', () => {
    assert.equal(hook.split('\n')[0], '#!/bin/sh');
    assert.match(hook, /^set -eu$/m);
    assert.match(hook, /\.tools\/gitleaks\/8\.30\.1\//);
    assert.match(hook, /git --pre-commit --staged --redact --exit-code 1 --config/);
    assert.doesNotMatch(hook, /\|\|\s*true|--no-verify|set -x/);
    assert.ok(!readFileSync(HOOK, 'utf8').includes('\r'), 'hook must use LF');
  });
  it('.gitattributes forces LF for hooks', () => {
    assert.match(read(join(REPO_ROOT, '.gitattributes')), /^\.githooks\/\* text eol=lf$/m);
  });
  it('config extends the default ruleset with no custom rules or allowlists', () => {
    const text = read(CONFIG);
    assert.match(text, /^\[extend\]\nuseDefault = true$/m);
    assert.doesNotMatch(text, /^\[\[rules\]\]|^\[\[?allowlists?\]?\]/m);
  });
  it('workflow scans the commit range of pull requests and pushes with the pinned tool', () => {
    const wf = read(WORKFLOW);
    assert.match(wf, /^on:\n {2}pull_request:\n {2}push:/m);
    assert.match(wf, /fetch-depth: 0/);
    assert.match(wf, /actions\/checkout@v4/);
    assert.match(wf, /GITLEAKS_VERSION: 8\.30\.1/);
    assert.match(wf, /GITLEAKS_SHA256: [a-f0-9]{64}/);
    assert.match(wf, /sha256sum --check --strict/);
    assert.match(wf, /0{40}/);
    assert.match(wf, /gitleaks git --redact .*--log-opts="\$\{GITLEAKS_RANGE\}"/);
    const code = wf
      .split('\n')
      .filter((line) => !/^\s*#/.test(line))
      .join('\n');
    assert.doesNotMatch(code, /pull_request_target|:\s*write\b/);
  });
});

describe('hook execution (stub gitleaks)', { skip: SKIP_SH }, () => {
  it('blocks a commit when gitleaks reports a different version', () => {
    const root = newRepoWithHook();
    const file = join(root, STUB_DIR, 'gitleaks');
    writeFile(root, `${STUB_DIR}/gitleaks`, '#!/bin/sh\nprintf "%s\\n" "0.0.0"\n');
    chmodSync(file, 0o755);
    const blocked = commit(root, { ...process.env, OS: 'Linux' });
    assert.notEqual(blocked.status, 0);
    assert.match(out(blocked), /invalid gitleaks version/);
  });

  it('blocks a commit when gitleaks is missing (no host PATH fallback)', () => {
    const root = newRepoWithHook();
    const bin = join(root, 'only-git');
    mkdirSync(bin);
    const quoted = "'" + GIT!.replaceAll("'", "'\\''") + "'";
    writeFile(root, 'only-git/git', `#!/bin/sh\nexec ${quoted} "$@"\n`);
    chmodSync(join(bin, 'git'), 0o755);
    // Only git and the POSIX shell (and its coreutils) are reachable: gitleaks is not.
    const shDir = SH ? dirname(SH) : '';
    const env: NodeJS.ProcessEnv = { ...process.env, PATH: [bin, shDir].filter(Boolean).join(delimiter) };
    if (process.platform === 'win32') delete env['Path'];
    const blocked = commit(root, env);
    assert.notEqual(blocked.status, 0);
    assert.match(out(blocked), /gitleaks 8\.30\.1 is missing/);
  });

  it('rejects a symlinked local tool', { skip: process.platform === 'win32' }, () => {
    const root = newRepoWithHook();
    writeFile(root, 'elsewhere/gitleaks', '#!/bin/sh\nprintf "%s\\n" "8.30.1"\n');
    chmodSync(join(root, 'elsewhere/gitleaks'), 0o755);
    mkdirSync(join(root, STUB_DIR), { recursive: true });
    symlinkSync(join(root, 'elsewhere/gitleaks'), join(root, STUB_DIR, 'gitleaks'));
    const blocked = commit(root, { ...process.env, OS: 'Linux' });
    assert.notEqual(blocked.status, 0);
    assert.match(out(blocked), /invalid local tool/);
  });

  it('runs the staged scan with the repo config when the version matches', () => {
    const root = newRepoWithHook();
    const file = join(root, STUB_DIR, 'gitleaks');
    writeFile(
      root,
      `${STUB_DIR}/gitleaks`,
      '#!/bin/sh\nif [ "$1" = "version" ]; then printf "%s\\n" "8.30.1"; exit 0; fi\nprintf "%s\\n" "$*" > "$(dirname "$0")/args.txt"\nexit 0\n',
    );
    chmodSync(file, 0o755);
    const ok = commit(root, { ...process.env, OS: 'Linux' });
    assert.equal(ok.status, 0, out(ok));
    const args = readFileSync(join(root, STUB_DIR, 'args.txt'), 'utf8');
    assert.match(
      args,
      /^git --pre-commit --staged --redact --exit-code 1 --config .*\.gitleaks\.toml/,
    );
  });
});
