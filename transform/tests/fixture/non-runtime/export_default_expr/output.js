const __re_export = global.__modules.import("module");
export * as rename from 'module';
global.__modules.export("test.js", { rename: __re_export });
