// copy binary from target dir to bin dir

import { fileURLToPath } from 'bun';
import path from 'node:path';
import { $ } from 'bun';

const is_release = process.argv.includes('--release');

const binary_name = 'uzumaki';
const core_folder_name = 'uzumaki';

const workspace_root = path.resolve(
  path.basename(fileURLToPath(import.meta.url)),
  '../',
);

const out_dir = is_release ? 'release' : 'debug';
const exe = path.join(workspace_root, `target/${out_dir}/${binary_name}.exe`);
const copyDir = path.join(workspace_root, `crates/${core_folder_name}/bin`);

await $`cp ${exe} ${copyDir}`;
