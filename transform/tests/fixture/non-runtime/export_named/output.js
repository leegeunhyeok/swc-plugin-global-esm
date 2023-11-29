const plain = 0;
const beforeRename = 1;
export { plain, beforeRename as afterRename };
global.__modules.init("test.js");
global.__modules.export("test.js", { plain, afterRename: beforeRename });
