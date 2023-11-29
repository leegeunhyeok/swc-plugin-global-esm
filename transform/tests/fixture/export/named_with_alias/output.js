const _module = global.__modules.import("module");
const __re_export = _module;
global.__modules.init("test.js");
global.__modules.export("test.js", { rename: __re_export });
