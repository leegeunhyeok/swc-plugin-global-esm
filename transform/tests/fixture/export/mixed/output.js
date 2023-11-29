const __module = global.__modules.import("@app/components");
const __module1 = global.__modules.import("@app/core");
const __module2 = global.__modules.import("@app/hooks");
const __module3 = global.__modules.import("react");
const React = __module3.default;
const useState = __module3.useState;
const useEffect = __module3.useEffect;
const Container = __module.Container;
const Section = __module.Section;
const Button = __module.Button;
const Text = __module.Text;
const useCustomHook = __module2.useCustomHook;
const app = __module1;
function MyComponent() {
  return null;
}
const __export_default = class {
  init() {
    // empty
  }
};
global.__modules.init("test.js");
global.__modules.export("test.js", {
  MyComponent,
  default: __export_default,
  app,
  useCustomHook
});
