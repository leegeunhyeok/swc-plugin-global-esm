const named = new Instance();
global.__modules.init("test.js");
global.__modules.export("test.js", { named });
