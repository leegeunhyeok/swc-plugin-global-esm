
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
   * Import with wildcard an exported module in global ESM context.
   */
  importWildcard(moduleName: string): ModuleExports;
  /**
   * Export a module to global ESM context.
   */
  export(moduleName: string, exports: ModuleExports): void;
  /**
   * Export all(*) module to global ESM context.
   */
  exportAll(moduleName: string, exports: ModuleExports): void;
}

((global, modules: Modules = {}) => {
  if (typeof global === 'undefined') {
    throw new Error('[Global ESM] `global` is undefined');
  }

  function getModule(moduleName: string) {
    return modules[moduleName] || (() => {
      throw new Error(`[Global ESM] "${moduleName}" module not found`);
    })();
  }

  function assertExports(moduleName: string, exports: unknown) {
    if (typeof modules[moduleName] !== 'object') {
      throw new Error(`[Global ESM] "${moduleName}" module not initialized`);
    }

    if (typeof exports !== 'object') {
      throw new Error(`[Global ESM] invalid exports argument on "${moduleName}" module registration`);
    }
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
      return getModule(moduleName);
    },
    importWildcard(moduleName) {
      const module = getModule(moduleName);
      const newModule = Object.create(null);

      Object.keys(module).forEach((moduleMember) => {
        if (moduleMember !== 'default' && Object.prototype.hasOwnProperty.call(module, moduleMember)) {
          const descriptor = Object.getOwnPropertyDescriptor(module, moduleMember);
          if (descriptor) {
            Object.defineProperty(
              newModule,
              moduleMember,
              descriptor
            );
          } else {
            newModule[moduleMember] = module[moduleMember];
          }
        }
      });
      return newModule;
    },
    export(moduleName, exports) {
      assertExports(moduleName, exports);
      Object.keys(exports).forEach((exportMember) => {
        if (Object.prototype.hasOwnProperty.call(exports, exportMember)) {
          Object.defineProperty(modules[moduleName], exportMember, {
            enumerable: true,
            get: () => exports[exportMember],
          });
        }
      });
    },
    exportAll(moduleName, exports) {
      assertExports(moduleName, exports);
      Object.keys(exports).forEach((exportMember) => {
        if (exportMember !== 'default' && Object.prototype.hasOwnProperty.call(exports, exportMember)) {
          Object.defineProperty(modules[moduleName], exportMember, {
            enumerable: true,
            get: () => exports[exportMember],
          });
        }
      });
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
