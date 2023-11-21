function namedFunction() {
  console.log('body');
}
global.__modules.export("test.js", { namedFunction });
