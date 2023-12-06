const _module = global.__modules.import("module");
const __re_export = _module.default;
global.__modules.init("test.js");
global.__modules.export("test.js", {
  default: __re_export,
});
