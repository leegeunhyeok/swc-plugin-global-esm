const a = global.__modules.import("module").a;
const b = global.__modules.import("module").b;
const c = global.__modules.import("module").c;
export { a, b, c } from 'module';
global.__modules.export("test.js", { a, b, c });
