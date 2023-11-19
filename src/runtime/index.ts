((global) => {
  const modules = {};
  Object.defineProperty(global, '__modules', {
    value: {
      import(moduleName: string) {
        return modules[moduleName] || (() => {
          throw new Error(`"${moduleName}" module not found`);
        })();
      },
      export(moduleName: string, exports: object) {
        modules[moduleName] = exports;
      },
    },
  });
})(
  typeof globalThis !== 'undefined'
    ? globalThis
    : typeof global !== 'undefined'
    ? global
    : typeof window !== 'undefined'
    ? window
    : this,
);
