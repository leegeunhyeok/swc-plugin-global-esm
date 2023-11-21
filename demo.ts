import { transform } from '@swc/core';
import highlight from 'cli-highlight';

const inputCode =`
import React, { useState, useEffect } from 'react';
import { Container, Section, Button, Text } from '@app/components';
import { useCustomHook } from '@app/hooks';

export interface MyComponentProps {
  message?: string;
}

export function MyComponent({ message = 'Hello, world!' }: MyComponentProps): JSX.Element {
  const [count, setCount] = useState(0);

  useEffect(() => {
    console.log('effect');
  }, []);

  useCustomHook();

  return (
    <Container>
      <Section>
        <Text>{message}</Text>
      </Section>
      <Section>
        <Text>{count}</Text>
      </Section>
      <Section>
        <Button onPress={() => setCount((v) => v + 1)}>
          <Text>{'Press Me'}</Text>
        </Button>
      </Section>
    </Container>
  );
};

export default class {}
`;

;(async () => {
  const { code: outputCode } = await transform(inputCode, {
    isModule: true,
    filename: 'demo.tsx',
    jsc: {
      target: 'es5',
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
