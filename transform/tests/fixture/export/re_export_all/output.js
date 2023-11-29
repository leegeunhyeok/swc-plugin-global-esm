const __re_export_all = global.__modules.importWildcard("module");
global.__modules.init("test.js");
global.__modules.exportAll("test.js", {
  ...__re_export_all
});
