import { build } from 'tsdown';
import path from 'node:path';

await build({
  entry: ['js/index.ts'],
  dts: { emitDtsOnly: true },
  outDir: 'dist',
  cwd: path.resolve('crates/uzumaki'),
});
