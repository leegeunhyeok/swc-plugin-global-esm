const __module = global.__modules.import("module");
const __default = __module.default;
global.__modules.init("test.js");
global.__modules.export("test.js", {
  default: __default,
});
