#!/usr/bin/env bun

import { fileURLToPath } from 'bun';
import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

type UzumakiConfig = {
  build?: {
    command?: string;
  };
  pack?: {
    dist?: string;
    entry?: string;
    output?: string;
    name?: string;
    baseBinary?: string;
  };
};

type BuildCliOptions = {
  configPath?: string;
  buildCommand?: string;
  dist?: string;
  entry?: string;
  output?: string;
  name?: string;
  baseBinary?: string;
  shouldRunBuild: boolean;
};

type LoadedConfig = {
  configPath: string;
  configDir: string;
  config: UzumakiConfig;
};

function color(text: string, value: string) {
  const start = Bun.color(value, 'ansi') ?? '';
  const reset = start ? '\u001B[0m' : '';
  return `${start}${text}${reset}`;
}

function bold(text: string) {
  return `\u001B[1m${text}\u001B[22m`;
}

function dim(text: string) {
  return `\u001B[2m${text}\u001B[22m`;
}

const TEMPLATE_DIR = path.resolve(
  path.dirname(fileURLToPath(new URL(import.meta.url))),
  'template',
);

const args = process.argv.slice(2);

const CONFIG_FILENAMES = [
  'uzumaki.config.ts',
  'uzumaki.config.js',
  'uzumaki.config.mjs',
  'uzumaki.config.json',
];

function help() {
  const commands = [
    {
      name: 'init',
      desc: 'Create a new Uzumaki project',
      args: '[directory]',
    },
    {
      name: 'run',
      desc: 'Run a JS/TS file in uzumaki runtime',
      args: './index.tsx [...args]',
    },
    {
      name: 'build',
      desc: 'Build an app from uzumaki.config.* and package it',
      args: '[--config <path>] [--no-build]',
    },
  ];

  const nameWidth = Math.max(...commands.map((cmd) => cmd.name.length));
  const argsWidth = Math.max(...commands.map((cmd) => cmd.args?.length ?? 0));

  const commandLines = commands
    .map((cmd) => {
      const name = bold(color(cmd.name.padEnd(nameWidth), '#60a5fa'));
      const argText = dim((cmd.args ?? '').padEnd(argsWidth));
      return `  ${name}  ${argText}  ${cmd.desc}`;
    })
    .join('\n');

  console.log(
    [
      `${bold(color('Uzumaki', '#60a5fa'))} Desktop UI Framework`,
      '',
      `${bold('Usage:')} uzumaki <command> ${dim('[...flags] [...args]')}`,
      '',
      `${bold('Commands:')}`,
      commandLines,
      '',
      `${bold('Build config:')} ${dim('uzumaki.config.ts | .js | .mjs | .json')}`,
    ].join('\n'),
  );
}

// for uzumaki developement
const BIN_FOLDER = path.resolve(
  path.dirname(fileURLToPath(new URL(import.meta.url))),
  '../../../target',
);

const require = createRequire(import.meta.url);

function getBinaryName() {
  switch (process.platform) {
    case 'win32': {
      return 'uzumaki.exe';
    }
    default: {
      return 'uzumaki';
    }
  }
}

function resolveTargetBinaryPath() {
  const binaryName = getBinaryName();
  const candidates = [
    path.join(BIN_FOLDER, 'release', binaryName),
    path.join(BIN_FOLDER, 'debug', binaryName),
  ];

  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }

  return candidates[0]!;
}

function getPlatformPackageName() {
  return `@uzumaki-apps/${process.platform}-${process.arch}`;
}

function resolvePackagedBinaryPath() {
  const packageName = getPlatformPackageName();
  try {
    const mod = require(packageName) as
      | string
      | { default?: string; binaryPath?: string; getBinaryPath?: () => string };

    if (typeof mod === 'string') {
      return mod;
    }

    if (typeof mod?.getBinaryPath === 'function') {
      return mod.getBinaryPath();
    }

    if (typeof mod?.binaryPath === 'string') {
      return mod.binaryPath;
    }

    if (typeof mod?.default === 'string') {
      return mod.default;
    }
  } catch {
    return null;
  }

  return null;
}

function resolveRuntimeBinaryPath() {
  const packagedBinaryPath = resolvePackagedBinaryPath();
  if (packagedBinaryPath && fs.existsSync(packagedBinaryPath)) {
    return packagedBinaryPath;
  }

  return resolveTargetBinaryPath();
}

async function run(entryPoint: string, extraArgs: string[] = []) {
  const binaryPath = resolveRuntimeBinaryPath();

  if (!fs.existsSync(binaryPath)) {
    console.error(
      [
        color('error:', '#ef4444'),
        `native binary not found at ${dim(binaryPath)}`,
      ].join(' '),
    );
    return 1;
  }

  const child = Bun.spawn([binaryPath, entryPoint, ...extraArgs], {
    stdin: 'inherit',
    stdout: 'inherit',
    stderr: 'inherit',
  });

  return await child.exited;
}

function parseBuildArgs(rawArgs: string[]): BuildCliOptions {
  const options: BuildCliOptions = {
    shouldRunBuild: true,
  };

  for (let i = 0; i < rawArgs.length; i += 1) {
    const arg = rawArgs[i]!;
    const next = rawArgs[i + 1];

    switch (arg) {
      case '--config': {
        if (!next) {
          throw new Error('--config requires a path');
        }
        options.configPath = next;
        i += 1;
        break;
      }
      case '--build-command': {
        if (!next) {
          throw new Error('--build-command requires a value');
        }
        options.buildCommand = next;
        i += 1;
        break;
      }
      case '--dist': {
        if (!next) {
          throw new Error('--dist requires a value');
        }
        options.dist = next;
        i += 1;
        break;
      }
      case '--entry': {
        if (!next) {
          throw new Error('--entry requires a value');
        }
        options.entry = next;
        i += 1;
        break;
      }
      case '--output':
      case '-o': {
        if (!next) {
          throw new Error(`${arg} requires a value`);
        }
        options.output = next;
        i += 1;
        break;
      }
      case '--name': {
        if (!next) {
          throw new Error('--name requires a value');
        }
        options.name = next;
        i += 1;
        break;
      }
      case '--base-binary': {
        if (!next) {
          throw new Error('--base-binary requires a value');
        }
        options.baseBinary = next;
        i += 1;
        break;
      }
      case '--no-build': {
        options.shouldRunBuild = false;
        break;
      }
      default: {
        throw new Error(`unknown build arg: ${arg}`);
      }
    }
  }

  return options;
}

function findConfigPath(startDir: string) {
  let currentDir = startDir;

  while (true) {
    for (const filename of CONFIG_FILENAMES) {
      const candidate = path.join(currentDir, filename);
      if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) {
        return candidate;
      }
    }

    const parent = path.dirname(currentDir);
    if (parent === currentDir) {
      return null;
    }
    currentDir = parent;
  }
}

async function loadConfig(configPath: string): Promise<UzumakiConfig> {
  const ext = path.extname(configPath);

  if (ext === '.json') {
    const raw = fs.readFileSync(configPath, 'utf8');
    return JSON.parse(raw) as UzumakiConfig;
  }

  const moduleUrl = pathToFileURL(configPath);
  moduleUrl.searchParams.set('t', `${Date.now()}`);
  const mod = (await import(moduleUrl.href)) as {
    default?: UzumakiConfig;
  } & UzumakiConfig;

  return (mod.default ?? mod) as UzumakiConfig;
}

async function resolveLoadedConfig(
  explicitConfigPath?: string,
): Promise<LoadedConfig> {
  const configPath = explicitConfigPath
    ? path.resolve(process.cwd(), explicitConfigPath)
    : findConfigPath(process.cwd());

  if (!configPath) {
    throw new Error(
      `could not find ${CONFIG_FILENAMES.join(', ')} from ${process.cwd()}`,
    );
  }

  if (!fs.existsSync(configPath)) {
    throw new Error(`config file not found: ${configPath}`);
  }

  const config = await loadConfig(configPath);
  if (!config || typeof config !== 'object' || Array.isArray(config)) {
    throw new Error(`config file must export an object: ${configPath}`);
  }

  return {
    configPath,
    configDir: path.dirname(configPath),
    config,
  };
}

function resolveFromConfigDir(configDir: string, value: string) {
  if (path.isAbsolute(value)) {
    return value;
  }

  return path.resolve(configDir, value);
}

async function runShellCommand(command: string, cwd: string) {
  const shellCmd =
    process.platform === 'win32'
      ? ['cmd.exe', '/d', '/s', '/c', command]
      : ['sh', '-lc', command];

  const child = Bun.spawn(shellCmd, {
    cwd,
    stdin: 'inherit',
    stdout: 'inherit',
    stderr: 'inherit',
  });

  return await child.exited;
}

async function buildApp(rawArgs: string[]) {
  const binaryPath = resolveRuntimeBinaryPath();
  if (!fs.existsSync(binaryPath)) {
    console.error(
      [
        color('error:', '#ef4444'),
        `native binary not found at ${dim(binaryPath)}`,
      ].join(' '),
    );
    return 1;
  }

  let options: BuildCliOptions;
  try {
    options = parseBuildArgs(rawArgs);
  } catch (error) {
    console.error(`${color('error:', '#ef4444')} ${(error as Error).message}`);
    console.error(
      `usage: ${dim(
        'uzumaki build [--config <path>] [--no-build] [--build-command <cmd>] [--dist <dir>] [--entry <rel>] [--output <exe>]',
      )}`,
    );
    return 1;
  }

  let loadedConfig: LoadedConfig;
  try {
    loadedConfig = await resolveLoadedConfig(options.configPath);
  } catch (error) {
    console.error(`${color('error:', '#ef4444')} ${(error as Error).message}`);
    return 1;
  }

  const buildCommand =
    options.buildCommand ?? loadedConfig.config.build?.command ?? null;

  if (options.shouldRunBuild && buildCommand) {
    const exitCode = await runShellCommand(
      buildCommand,
      loadedConfig.configDir,
    );
    if (exitCode !== 0) {
      return exitCode;
    }
  }

  const distValue = options.dist ?? loadedConfig.config.pack?.dist;
  const entryValue = options.entry ?? loadedConfig.config.pack?.entry;
  const outputValue = options.output ?? loadedConfig.config.pack?.output;
  const nameValue = options.name ?? loadedConfig.config.pack?.name;
  const baseBinaryValue =
    options.baseBinary ?? loadedConfig.config.pack?.baseBinary;

  if (!distValue) {
    console.error(`${color('error:', '#ef4444')} missing pack.dist`);
    return 1;
  }

  if (!entryValue) {
    console.error(`${color('error:', '#ef4444')} missing pack.entry`);
    return 1;
  }

  if (!outputValue) {
    console.error(`${color('error:', '#ef4444')} missing pack.output`);
    return 1;
  }

  const distPath = resolveFromConfigDir(loadedConfig.configDir, distValue);
  const resolvedOutput = resolveFromConfigDir(
    loadedConfig.configDir,
    outputValue,
  );
  const outputPath =
    process.platform === 'win32' && !resolvedOutput.endsWith('.exe')
      ? `${resolvedOutput}.exe`
      : resolvedOutput;
  const entryPath = path.join(distPath, entryValue);
  const baseBinaryPath = baseBinaryValue
    ? resolveFromConfigDir(loadedConfig.configDir, baseBinaryValue)
    : null;

  if (!fs.existsSync(distPath)) {
    console.error(
      `${color('error:', '#ef4444')} dist directory not found: ${distPath}`,
    );
    return 1;
  }

  if (!fs.existsSync(entryPath)) {
    console.error(
      `${color('error:', '#ef4444')} entry file not found: ${entryPath}`,
    );
    return 1;
  }

  const packArgs = [
    'pack',
    '--dist',
    distPath,
    '--entry',
    entryValue,
    '--output',
    outputPath,
  ];

  if (nameValue) {
    packArgs.push('--name', nameValue);
  }

  if (baseBinaryPath) {
    packArgs.push('--base-binary', baseBinaryPath);
  }

  const child = Bun.spawn([binaryPath, ...packArgs], {
    cwd: loadedConfig.configDir,
    stdin: 'inherit',
    stdout: 'inherit',
    stderr: 'inherit',
  });

  return await child.exited;
}

function copyDirSync(src: string, dest: string) {
  fs.mkdirSync(dest, { recursive: true });
  for (const entry of fs.readdirSync(src, { withFileTypes: true })) {
    const srcPath = path.join(src, entry.name);
    const destPath = path.join(dest, entry.name);
    if (entry.isDirectory()) {
      copyDirSync(srcPath, destPath);
    } else {
      fs.copyFileSync(srcPath, destPath);
    }
  }
}

function replaceTemplateVars(filePath: string, vars: Record<string, string>) {
  let content = fs.readFileSync(filePath, 'utf8');
  for (const [key, value] of Object.entries(vars)) {
    content = content.replaceAll(`{{${key}}}`, value);
  }
  fs.writeFileSync(filePath, content, 'utf8');
}

async function initProject(targetArg?: string) {
  if (process.platform === 'linux') {
    console.error(
      `${color('error:', '#ef4444')} Linux is not supported yet. Linux support is on the way!`,
    );
    return 1;
  }

  const projectDir = path.resolve(process.cwd(), targetArg ?? '.');
  const projectName = path.basename(projectDir);

  if (fs.existsSync(projectDir) && fs.readdirSync(projectDir).length > 0) {
    console.error(
      `${color('error:', '#ef4444')} directory ${dim(projectDir)} is not empty`,
    );
    return 1;
  }

  console.log(
    `\n${bold(color('Uzumaki', '#60a5fa'))} Creating project ${bold(projectName)}...\n`,
  );

  copyDirSync(TEMPLATE_DIR, projectDir);

  const templateFiles = [
    path.join(projectDir, 'package.json'),
    path.join(projectDir, 'uzumaki.config.ts'),
    path.join(projectDir, 'src', 'index.tsx'),
  ];

  for (const file of templateFiles) {
    replaceTemplateVars(file, { PROJECT_NAME: projectName });
  }

  console.log(`  ${color('created', '#22c55e')} ${projectName}/package.json`);
  console.log(`  ${color('created', '#22c55e')} ${projectName}/tsconfig.json`);
  console.log(
    `  ${color('created', '#22c55e')} ${projectName}/uzumaki.config.ts`,
  );
  console.log(`  ${color('created', '#22c55e')} ${projectName}/src/index.tsx`);

  console.log(`\n${bold('Prerequisites:')}\n`);
  console.log(`  ${dim('Bun is required for building.')} https://bun.sh`);

  console.log(`\n${bold('Next steps:')}\n`);
  if (targetArg) {
    console.log(`  cd ${projectName}`);
  }
  console.log('  pnpm install');
  console.log('  pnpm dev');
  console.log('');

  return 0;
}

async function main() {
  if (args.length === 0) {
    help();
    return 0;
  }

  const cmd = args[0]!;
  switch (cmd) {
    case 'init': {
      return await initProject(args[1]);
    }

    case 'run': {
      const entryPoint = args[1];
      if (!entryPoint) {
        console.error(`${color('error:', '#ef4444')} entry point not provided`);
        console.error(`usage: ${dim('uzumaki run <entrypoint> [...args]')}`);
        return 1;
      }
      return await run(entryPoint, args.slice(2));
    }

    case 'build': {
      return await buildApp(args.slice(1));
    }

    default: {
      return await run(cmd, args.slice(1));
    }
  }
}

const exitCode = await main();
if (exitCode !== 0) {
  process.exit(exitCode);
}
