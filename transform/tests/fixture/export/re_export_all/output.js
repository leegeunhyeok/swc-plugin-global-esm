const __module = global.__modules.import("module");
const __re_export_all = __module;
global.__modules.init("test.js");
global.__modules.export("test.js", {}, {
  ...__re_export_all
});
