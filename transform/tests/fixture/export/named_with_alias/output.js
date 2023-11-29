const __re_export = global.__modules.importWildcard("module");
global.__modules.init("test.js");
global.__modules.export("test.js", { rename: __re_export });
