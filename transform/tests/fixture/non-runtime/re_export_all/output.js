const __re_export_all = global.__modules.import("module");
export * from 'module';
global.__modules.export("test.js", { ...__re_export_all });
