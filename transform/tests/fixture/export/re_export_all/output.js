const _module = global.__modules.import("module");
const __re_export_all = _module;
global.__modules.init("test.js");
global.__modules.export("test.js", {}, {
  ...__re_export_all
});
