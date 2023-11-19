import { transform } from '@swc/core';
import highlight from 'cli-highlight';

const inputCode =`
import React, { useState, useEffect } from 'react';
import { Container, Section, Button, Text } from '@app/components';
import { useCustomHook } from '@app/hooks';

export function MyComponent (): JSX.Element {
  const [count, setCount] = useState(0);

  useEffect(() => {
    console.log('effect');
  }, []);

  useCustomHook();

  return (
    <Container>
      <Section>
        <Text>{'Hello, World'}</Text>
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
      parser: {
        syntax: 'typescript',
        tsx: true,
      },
      experimental: {
        plugins: [
          ['.', { runtimeModule: true }],
        ],
      },
      externalHelpers: false,
    },
  });

  console.log(highlight(outputCode, { language: 'js' }));
})();
