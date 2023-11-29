import path from 'node:path';
import { transform } from '@swc/core';
import { faker } from '@faker-js/faker';
import '../index';

const SNAPSHOT_TEST_CODE_INPUT = `
import React, { useState } from 'react';
import { Container } from '@app/components';
import { useCustomHook } from '@app/hooks';
import * as app from '@app/core';

// named export & declaration
export function MyComponent (): JSX.Element {
  const [count, setCount] = useState(0);
  useCustomHook(app);
  return <Container>{count}</Container>;
}

// export with alias
export { app as APP };

// default export & anonymous declaration
export default class {}

// re-exports
export * from '@app/module_a';
export * as B from '@app/module_b';
export { c as C } from '@app/module_c'; 
`;

const generateModulePath = () => {
  return path.join(faker.system.directoryPath(), faker.word.noun() + '.js');
};

describe('swc-plugin-global-esm/runtime', () => {
  const snapshot = process.env.CI === 'true' ? it.skip : it;

  beforeEach(() => {
    global.__modules.reset();
  });

  snapshot('match snapshot', async () => {
    const { code: outputCode } = await transform(SNAPSHOT_TEST_CODE_INPUT, {
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
    expect(outputCode).toMatchSnapshot();
  });

  it('global object must have `__modules` exposed', () => {
    expect(typeof global.__modules === 'object').toEqual(true);
  });

  describe('initialize modules', () => {
    let modulePath: string;

    beforeEach(() => {
      modulePath = path.join(faker.system.directoryPath(), faker.word.noun() + '.js');
    });

    describe('when call `export()` before `init()`', () => {
      it('should throw error', () => {
        expect(() => global.__modules.export(modulePath, {})).toThrow(Error);
      });
    });

    describe('when call `export()` after `init()`', () => {
      it('should not throw error', () => {
        global.__modules.init(modulePath);
        expect(() => global.__modules.export(modulePath, {})).not.toThrow();
      });
    });
  });

  describe('reset modules', () => {
    let modules: string[];

    beforeEach(() => {
      modules = [
        generateModulePath(),
        generateModulePath(),
        generateModulePath(),
        generateModulePath(),
        generateModulePath(),
      ];

      // Register dummy exports
      modules.forEach((modulePath) => {
        global.__modules.init(modulePath);
        global.__modules.export(modulePath, {});
      });
    });

    describe('when call `reset()` with module name argument', () => {
      let targetModule: string;

      beforeEach(() => {
        targetModule = faker.helpers.arrayElement(modules);
        global.__modules.reset(targetModule);
      });

      describe('when call `import()` the reset module', () => {
        it('should reset specified module only', () => {
          expect(() => global.__modules.import(targetModule)).toThrow(Error);
          modules
            .filter((modulePath) => modulePath !== targetModule)
            .forEach((modulePath) => {
              expect(() => global.__modules.import(modulePath)).not.toThrow();
            });
        });
      });
    });

    describe('when call `reset()` without any arguments', () => {
      beforeEach(() => {
        global.__modules.reset();
      });

      describe('when call `import()` the all of reset modules', () => {
        it('should reset all modules', () => {
          modules.forEach((modulePath) => {
            expect(() => global.__modules.import(modulePath)).toThrow(Error);
          });
        });
      });
    });
  });

  describe('export & import modules', () => {
    let modulePath: string;

    beforeEach(() => {
      modulePath = generateModulePath();
    });

    describe('when call `import()` with unexported module', () => {
      it('should throw error', () => {
        expect(() => global.__modules.import(faker.string.alpha())).toThrow(Error);
      });
    });

    describe('when call `export()` with invalid arguments', () => {
      beforeEach(() => {
        global.__modules.init(modulePath);
      });

      describe('when `exports` is invalid', () => {
        let invalidExports: any;

        beforeEach(() => {
          invalidExports = faker.helpers.arrayElement([
            null,
            undefined,
            faker.number.int(),
            faker.string.alphanumeric(),
          ]);
        });

        it('should throw error', () => {
          expect(() => global.__modules.export(modulePath, invalidExports)).toThrow(Error);
        });
      });
    });

    describe('when call `exportAll()` with invalid arguments', () => {
      beforeEach(() => {
        global.__modules.init(modulePath);
      });

      describe('when `exports` is invalid', () => {
        let invalidExports: any;

        beforeEach(() => {
          invalidExports = faker.helpers.arrayElement([
            null,
            undefined,
            faker.number.int(),
            faker.string.alphanumeric(),
          ]);
        });

        it('should throw error', () => {
          expect(() => global.__modules.exportAll(modulePath, invalidExports)).toThrow(Error);
        });
      });
    });

    describe('when call `export()` with valid exports object', () => {
      let exportKey: string;
      let exportValue: string;
      let exports: Record<string, unknown>;

      beforeEach(() => {
        exportKey = faker.string.alpha(10);
        exportValue = faker.string.uuid();
        exports = { [exportKey]: exportValue };
        global.__modules.init(modulePath);
        global.__modules.export(modulePath, exports);
      });

      describe('when call `import()` with the exported module', () => {
        it('should match the exported module', () => {
          const exportedModule = global.__modules.import(modulePath);
          expect(exportedModule[exportKey]).toEqual(exportValue);
        });
      });
    });

    describe('when call `export()` with valid exports object that has `default` property', () => {
      let exportValue: string;
      let exports: Record<string, unknown>;

      beforeEach(() => {
        exportValue = faker.string.uuid();
        exports = { default: exportValue };
        global.__modules.init(modulePath);
        global.__modules.export(modulePath, exports);
      });

      describe('when call `importWildcard()` with the exported module', () => {
        it('should exclude `default` property', () => {
          const exportedModule = global.__modules.importWildcard(modulePath);
          expect(exports.default).toEqual(exportValue);
          expect(exportedModule.default).toBeUndefined();
        });
      });
    });

    describe('when call `exportAll()` with valid argument', () => {
      let exportKey: string;
      let exportValue: string;
      let exportAll: Record<string, unknown>;

      beforeEach(() => {
        exportKey = faker.string.alpha(10);
        exportValue = faker.string.uuid();
        exportAll = { [exportKey]: exportValue };
        global.__modules.init(modulePath);
        global.__modules.exportAll(modulePath, exportAll);
      });

      describe('when call `import()` with the exported module', () => {
        it('should match the exported module', () => {
          const exportedModule = global.__modules.import(modulePath);
          expect(exportedModule[exportKey]).toEqual(exportValue);
        });
      });

      describe('when `exportAll` object contains `default` property', () => {
        beforeEach(() => {
          exportValue = faker.string.uuid();
          exportAll = { default: exportValue };
          global.__modules.init(modulePath);
          global.__modules.exportAll(modulePath, exportAll);
        });

        describe('when call `import()` with the exported module', () => {
          it('should exclude `default` property', () => {
            const exportedModule = global.__modules.import(modulePath);
            expect(exportAll.default).toEqual(exportValue);
            expect(exportedModule.default).toBeUndefined();
          });
        });
      });
    });
  });
});
