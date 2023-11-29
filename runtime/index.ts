
type Modules<ModuleName extends string = string> = Record<ModuleName, ModuleExports | undefined>;
type ModuleExports<ExportMember extends string = string> = Record<ExportMember, unknown>;

export interface GlobalEsModule {
  /**
   * Reset all modules or reset specified module if `moduleName` is provided.
   */
  reset(moduleName?: string): void;
  /**
   * Initialize module before exports.
   */
  init(moduleName: string): void;
  /**
   * Import an exported module in global ESM context.
   */
  import(moduleName: string): ModuleExports;
  /**
   * Export a module to global ESM context.
   */
  export(moduleName: string, exports: ModuleExports, reExports?: ModuleExports): void;
}

((global, modules: Modules = {}) => {
  if (typeof global === 'undefined') {
    throw new Error('[Global ESM] `global` is undefined');
  }

  const globalEsmApi: GlobalEsModule = {
    reset(moduleName) {
      if (typeof moduleName === 'string') {
        modules[moduleName] = undefined;
      } else {
        modules = {};
      }
    },
    init(moduleName) {
      modules[moduleName] = Object.create(null);
    },
    import(moduleName) {
      return modules[moduleName] || (() => {
        throw new Error(`[Global ESM] "${moduleName}" module not found`);
      })();
    },
    export(moduleName, exports, exportAll) {
      if (typeof modules[moduleName] !== 'object') {
        throw new Error(`[Global ESM] "${moduleName}" module not initialized`);
      }

      if (typeof exports !== 'object') {
        throw new Error(`[Global ESM] invalid exports argument on "${moduleName}" module registration`);
      }

      Object.keys(exports).forEach((exportMember) => {
        if (Object.prototype.hasOwnProperty.call(exports, exportMember)) {
          Object.defineProperty(modules[moduleName], exportMember, {
            enumerable: true,
            get: () => exports[exportMember],
          });
        }
      });

      if (typeof exportAll === 'object') {
        Object.keys(exportAll).forEach((reExportMember) => {
          if (reExportMember !== 'default' && Object.prototype.hasOwnProperty.call(exportAll, reExportMember)) {
            Object.defineProperty(modules[moduleName], reExportMember, {
              enumerable: true,
              get: () => exportAll[reExportMember],
            });
          }
        });
      }
    },
  };

  Object.defineProperty(global, '__modules', { value: globalEsmApi });

  // Define `global` property to global object.
  if (!('global' in global)) {
    Object.defineProperty(global, 'global', { value: global });
  }
})(
  typeof globalThis !== 'undefined'
    ? globalThis
    : typeof global !== 'undefined'
    ? global
    : typeof window !== 'undefined'
    ? window
    : this,
);
