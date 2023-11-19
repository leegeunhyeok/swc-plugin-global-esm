# swc-plugin-global-esm

> [!WARNING]
> This plugin is for custom module system to implement Hot Module Replacement(HMR) in some bundlers that don't support it.

## Installation

```bash
npm install swc-plugin-global-esm
# or yarn
yarn add swc-plugin-global-esm
```

## Usage

Inject runtime script to top of bundle.

```ts
import 'swc-plugin-global-esm/runtime';

// Now you can use global module API (global.__modules)
```

and add plugin to your swc options.

```ts
import { transform } from '@swc/core';

await transform(code, {
  jsc: {
    experimental: {
      plugins: [
        // Add plugin here.
        ['swc-plugin-global-esm', {
          /**
           * Convert import statements to custom module system and remove export statements.
           *
           * Defaults to `false`.
           */
          runtimeModule: true,
        }],
      ],
    },
    /**
     * You should disable external helpers when `runtimeModule` is `true`
     * because external helper import statements will be added after plugin transformation.
     */
    externalHelpers: false,
  },
});
```

## Preview

Before

```ts
import React, { useState, useEffect } from 'react';
import { Container, Section, Button, Text } from '@app/components';
import { useCustomHook } from '@app/hooks';
import * as app from '@app/core';

export function MyComponent (): JSX.Element {
  // ...
}

// anonymous class
export default class {}
```

After

```js
// with `runtimeModule: true`
var React = global.__modules.import("react").default;
var useState = global.__modules.import("react").useState;
var useEffect = global.__modules.import("react").useEffect;
var Container = global.__modules.import("@app/components").Container;
var Section = global.__modules.import("@app/components").Section;
var Button = global.__modules.import("@app/components").Button;
var Text = global.__modules.import("@app/components").Text;
var useCustomHook = global.__modules.import("@app/hooks").useCustomHook;
var app = global.__modules.import("@app/core");

function MyComponent () {
  // ...
}

var __export_default = class {}

global.__modules.export("<module-file-name>", {
  "MyComponent": MyComponent,
  "default": __export_default
});
```

## License

[MIT](./LICENSE)
