import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, writeFile, rm, mkdir, symlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, dirname, basename } from 'node:path';
import {
  selectGitleaksAsset,
  installGitleaksAsset,
  verifyGitleaksArchive,
  GITLEAKS_VERSION,
} from './setup-gitleaks.mts';

const cleanup = async (root: string) => {
  assert.equal(dirname(root), tmpdir());
  assert.ok(basename(root).startsWith('dd-gitleaks-'));
  await rm(root, { recursive: true, force: true });
};
test('native assets are pinned to official HTTPS releases and unsupported platforms fail closed', () => {
  for (const platform of ['win32', 'linux']) {
    const asset = selectGitleaksAsset(platform, 'x64');
    assert.match(
      asset.url,
      /^https:\/\/github\.com\/gitleaks\/gitleaks\/releases\/download\/v8\.30\.1\//,
    );
    assert.match(asset.sha256, /^[a-f0-9]{64}$/);
  }
  assert.equal(GITLEAKS_VERSION, '8.30.1');
  assert.throws(() => selectGitleaksAsset('darwin', 'arm64'));
});
test('archive hash is checked before extraction or execution', async () => {
  assert.throws(() => verifyGitleaksArchive(Buffer.from('invalid'), 'a'.repeat(64)));
  const root = await mkdtemp(join(tmpdir(), 'dd-gitleaks-invalid-'));
  try {
    let touched = false;
    await assert.rejects(
      installGitleaksAsset(root, selectGitleaksAsset('win32', 'x64'), {
        download: async () => Buffer.from('invalid'),
        extract: async () => {
          touched = true;
        },
        version: () => {
          touched = true;
          return '8.30.1';
        },
      }),
    );
    assert.equal(touched, false);
  } finally {
    await cleanup(root);
  }
});
test('verified native executable is promoted atomically only with the exact version', async () => {
  const root = await mkdtemp(join(tmpdir(), 'dd-gitleaks-install-'));
  const archive = Buffer.from('synthetic offline archive');
  const asset = {
    ...selectGitleaksAsset('win32', 'x64'),
    sha256: createHash('sha256').update(archive).digest('hex'),
  };
  try {
    const ops = {
      download: async () => archive,
      extract: async (_file: string, folder: string, binary: string) => {
        await writeFile(join(folder, binary), 'verified native fixture');
      },
      version: () => '8.30.1',
    };
    const installed = await installGitleaksAsset(root, asset, ops);
    assert.equal(await readFile(installed, 'utf8'), 'verified native fixture');
    await assert.rejects(installGitleaksAsset(root, asset, { ...ops, version: () => '0.0.0' }));
    assert.equal(await readFile(installed, 'utf8'), 'verified native fixture');
  } finally {
    await cleanup(root);
  }
});
test('installer refuses symlinked tool roots before downloading', async () => {
  const root = await mkdtemp(join(tmpdir(), 'dd-gitleaks-path-'));
  try {
    await mkdir(join(root, 'other'));
    await symlink(join(root, 'other'), join(root, '.tools'), 'junction');
    let downloaded = false;
    await assert.rejects(
      installGitleaksAsset(root, selectGitleaksAsset('win32', 'x64'), {
        download: async () => {
          downloaded = true;
          return Buffer.alloc(0);
        },
      }),
    );
    assert.equal(downloaded, false);
  } finally {
    await cleanup(root);
  }
});
test('macOS is not served by a downloaded asset (PATH only) and other arches fail closed', () => {
  assert.throws(() => selectGitleaksAsset('darwin', 'x64'), /unsupported-platform/);
  assert.throws(() => selectGitleaksAsset('linux', 'arm64'), /unsupported-platform/);
  assert.throws(() => selectGitleaksAsset('win32', 'ia32'), /unsupported-platform/);
});
test('assets outside the official release origin or with unknown binaries are rejected', async () => {
  const root = await mkdtemp(join(tmpdir(), 'dd-gitleaks-asset-'));
  try {
    const good = selectGitleaksAsset('linux', 'x64');
    let downloaded = false;
    const download = async () => {
      downloaded = true;
      return Buffer.alloc(0);
    };
    await assert.rejects(
      installGitleaksAsset(
        root,
        { ...good, url: 'https://example.invalid/gitleaks.tar.gz' },
        { download },
      ),
      /invalid-asset/,
    );
    await assert.rejects(
      installGitleaksAsset(root, { ...good, binary: '../evil' }, { download }),
      /invalid-asset/,
    );
    assert.equal(downloaded, false);
  } finally {
    await cleanup(root);
  }
});
test('a binary reporting another version is never promoted', async () => {
  const root = await mkdtemp(join(tmpdir(), 'dd-gitleaks-version-'));
  const archive = Buffer.from('synthetic offline archive');
  const asset = {
    ...selectGitleaksAsset('linux', 'x64'),
    sha256: createHash('sha256').update(archive).digest('hex'),
  };
  try {
    await assert.rejects(
      installGitleaksAsset(root, asset, {
        download: async () => archive,
        extract: async (_file: string, folder: string, binary: string) => {
          await writeFile(join(folder, binary), 'fixture');
        },
        version: () => '8.0.0',
      }),
      /version-mismatch/,
    );
    await assert.rejects(readFile(join(root, '.tools', 'gitleaks', GITLEAKS_VERSION, 'gitleaks')));
  } finally {
    await cleanup(root);
  }
});
test('checksum mismatch is rejected with a stable error', () => {
  assert.throws(() => verifyGitleaksArchive(Buffer.from('x'), 'not-a-hash'), /checksum-mismatch/);
});
