const plain = 0;
const beforeRename = 1;
global.__modules.export("test.js", { plain, afterRename: beforeRename });
