#!/usr/bin/env bun

import { execSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const workspaceRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  '..',
);

const platforms = [
  {
    id: 'win32-x64',
    binary: 'uzumaki.exe',
    packageName: '@uzumaki-apps/win32-x64',
  },
  {
    id: 'darwin-x64',
    binary: 'uzumaki',
    packageName: '@uzumaki-apps/darwin-x64',
  },
  {
    id: 'darwin-arm64',
    binary: 'uzumaki',
    packageName: '@uzumaki-apps/darwin-arm64',
  },
];

function parseArgs() {
  const argv = process.argv.slice(2);
  let version: string | undefined;
  let artifactsDir: string | undefined;
  let dryRun = false;

  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i]!;
    const next = argv[i + 1];
    switch (arg) {
      case '--version': {
        version = next;
        i++;
        break;
      }
      case '--artifacts': {
        artifactsDir = next;
        i++;
        break;
      }
      case '--dry-run': {
        dryRun = true;
        break;
      }
      default: {
        throw new Error(`unknown arg: ${arg}`);
      }
    }
  }

  if (!version) throw new Error('--version is required');
  if (!artifactsDir) throw new Error('--artifacts is required');

  return {
    version,
    artifactsDir: path.resolve(workspaceRoot, artifactsDir),
    dryRun,
  };
}

function run(cmd: string, cwd?: string) {
  console.log(`$ ${cmd}`);
  execSync(cmd, { cwd: cwd ?? workspaceRoot, stdio: 'inherit' });
}

function generatePlatformPackages(artifactsDir: string, version: string) {
  for (const platform of platforms) {
    const binaryPath = path.join(artifactsDir, platform.id, platform.binary);
    if (!fs.existsSync(binaryPath)) {
      throw new Error(`binary not found: ${binaryPath}`);
    }
    run(
      `bun scripts/copy-artifacts.ts --platform ${platform.id} --version ${version} --binary ${binaryPath}`,
    );
  }
}

function prepareMainPackage(version: string) {
  const pkgPath = path.join(workspaceRoot, 'crates', 'uzumaki', 'package.json');
  const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));

  pkg.version = version;
  pkg.optionalDependencies = Object.fromEntries(
    platforms.map((p) => [p.packageName, version]),
  );

  fs.writeFileSync(pkgPath, `${JSON.stringify(pkg, null, 2)}\n`);
  console.log(`updated uzumaki-ui package.json to v${version}`);
}

function publishPackages(dryRun: boolean) {
  if (dryRun) {
    console.log('dry run — skipping publish');
    return;
  }

  for (const platform of platforms) {
    const dir = path.join(workspaceRoot, 'npm', platform.id);
    if (!fs.existsSync(path.join(dir, 'package.json'))) {
      throw new Error(`package.json not found in ${dir}`);
    }
    run('pnpm publish --access public --no-git-checks', dir);
  }

  run(
    'pnpm publish --access public --no-git-checks',
    path.join(workspaceRoot, 'crates', 'uzumaki'),
  );
}

function main() {
  const { version, artifactsDir, dryRun } = parseArgs();

  console.log(`\npreparing release v${version}\n`);

  generatePlatformPackages(artifactsDir, version);
  prepareMainPackage(version);
  publishPackages(dryRun);

  console.log(`\n${dryRun ? 'dry run complete' : `published v${version}`}\n`);
}

main();
