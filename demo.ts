import { transform } from '@swc/core';
import highlight from 'cli-highlight';

const inputCode =`
import React, { useState, useEffect } from 'react';
import { Container } from '@app/components';
import { useCustomHook } from '@app/hooks';
import * as app from '@app/core';

// named export & declaration
export function MyComponent(): JSX.Element {
  const [count, setCount] = useState(0);
  useCustomHook(app);
  return <Container>{count}</Container>;
}

// named export with alias
export { app as AppCore };

// default export & anonymous declaration
export default class {}

// re-exports
export * from '@app/module_a';
export * from '@app/module_b';
export * as car from '@app/module_c';
export { driver as driverModule } from '@app/module_d';
`;

;(async () => {
  const { code: outputCode } = await transform(inputCode, {
    isModule: true,
    filename: 'demo.tsx',
    jsc: {
      target: 'esnext',
      parser: {
        syntax: 'typescript',
        tsx: true,
      },
      experimental: {
        plugins: [
          ['.', {
            runtimeModule: true,
            importPaths: {
              react: 'node_modules/react/cjs/react.development.js',
            },
          }],
        ],
      },
      externalHelpers: false,
    },
  });

  console.log(highlight(outputCode, { language: 'js' }));
})();
