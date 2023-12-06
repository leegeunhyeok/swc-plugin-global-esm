const _module = global.__modules.import("module");
const __re_export = _module.a;
const __re_export1 = _module.b;
const __re_export2 = _module.c;
global.__modules.init("test.js");
global.__modules.export("test.js", {
  a: __re_export,
  b: __re_export1,
  c: __re_export2
});
