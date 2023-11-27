const __default = global.__modules.import("a").default;
const __default1 = global.__modules.import("b").default;
global.__modules.export("test.js", {
  A: __default,
  B: __default1
});
