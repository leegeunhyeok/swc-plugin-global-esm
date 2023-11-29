import { a } from "module";
import { b } from "module";
import { c } from "module";
export { a, b, c } from 'module';
global.__modules.init("test.js");
global.__modules.export("test.js", { a, b, c });
