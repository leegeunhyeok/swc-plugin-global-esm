const _module = global.__modules.import("module");
const __default = _module.default;
global.__modules.init("test.js");
global.__modules.export("test.js", {
  default: __default,
});
