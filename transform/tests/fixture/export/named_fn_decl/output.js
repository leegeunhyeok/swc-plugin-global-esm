function namedFunction() {
  console.log('body');
}
global.__modules.init("test.js");
global.__modules.export("test.js", { namedFunction });
