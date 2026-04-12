export default {
  build: {
    command: 'bun build src/index.tsx --target node --outdir dist --minify',
  },
  pack: {
    dist: './dist',
    entry: 'index.js',
    output: './{{PROJECT_NAME}}',
    name: '{{PROJECT_NAME}}',
  },
};
