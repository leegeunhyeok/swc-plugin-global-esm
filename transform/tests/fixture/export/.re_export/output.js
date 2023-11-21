const a = global.__modules.import("module").a;
const b = global.__modules.import("module").b;
const c = global.__modules.import("module").c;
global.__modules.export("test.js", { a, b, c });
