const __app_components = global.__modules.import("@app/components");
const __app_hooks = global.__modules.import("@app/hooks");
const _react = global.__modules.import("react");
const React = _react.default;
const useState = _react.useState;
const useEffect = _react.useEffect;
const Container = __app_components.Container;
const Section = __app_components.Section;
const Button = __app_components.Button;
const Text = __app_components.Text;
const useCustomHook = __app_hooks.useCustomHook;
const app = global.__modules.importWildcard("@app/core");
function MyComponent() {
  return null;
}
class __Class {
  init() {
    // empty
  }
}
global.__modules.init("test.js");
global.__modules.export("test.js", {
  MyComponent,
  default: __Class,
  app,
  useCustomHook
});
