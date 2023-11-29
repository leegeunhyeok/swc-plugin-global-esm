const _module = global.__modules.import("module");
const a = _module.a;
const b = _module.b;
const c = _module.c;
global.__modules.init("test.js");
global.__modules.export("test.js", { a, b, c });
