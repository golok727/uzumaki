// copy binary from target dir to bin dir

import { fileURLToPath } from 'bun';
import path from 'node:path';
import { $ } from 'bun';

const core_folder_name = 'uzumaki_core_exp';
const workspace_root = path.resolve(
  path.basename(fileURLToPath(import.meta.url)),
  '../',
);

const exe = path.join(workspace_root, `target/debug/${core_folder_name}.exe`);
const copyDir = path.join(workspace_root, `crates/${core_folder_name}/bin`);

await $`cp ${exe} ${copyDir}`;
