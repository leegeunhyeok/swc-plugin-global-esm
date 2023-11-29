const __module = global.__modules.import("module");
const a = __module.a;
const b = __module.b;
const c = __module.c;
global.__modules.init("test.js");
global.__modules.export("test.js", { a, b, c });
