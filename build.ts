import path from 'node:path';
import * as esbuild from 'esbuild';
import { name, version } from './package.json';

esbuild.build({
  entryPoints: [path.resolve('src/runtime/index.ts')],
  outfile: 'dist/runtime.js',
  banner: {
    js: `// ${name}@${version} runtime`
  },
}).catch((error) => {
  console.error(error);
  process.exit(1);
});
