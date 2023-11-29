const _a = global.__modules.import("a");
const _b = global.__modules.import("b");
const __default = _a.default;
const __default1 = _b.default;
global.__modules.init("test.js");
global.__modules.export("test.js", {
  A: __default,
  B: __default1
});
