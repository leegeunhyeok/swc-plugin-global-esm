const __module = global.__modules.import("a");
const __module1 = global.__modules.import("b");
const __default = __module.default;
const __default1 = __module1.default;
global.__modules.init("test.js");
global.__modules.export("test.js", {
  A: __default,
  B: __default1
});
