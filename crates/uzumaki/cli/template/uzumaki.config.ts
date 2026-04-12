const isWindows = process.platform === 'win32';

export default {
  build: {
    command: 'bun build src/index.tsx --target node --outdir dist --minify',
  },
  pack: {
    dist: './dist',
    entry: 'index.js',
    output: isWindows ? './{{PROJECT_NAME}}.exe' : './{{PROJECT_NAME}}',
    name: '{{PROJECT_NAME}}',
  },
};
