const __module = global.__modules.import("dummy");
const __dummy = __module.default;
global.__modules.init("test.js");
global.__modules.export("test.js", {});
