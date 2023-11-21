const __export_named = global.__modules.import("module");
export * as rename from 'module';
global.__modules.export("test.js", { rename: __export_named });
