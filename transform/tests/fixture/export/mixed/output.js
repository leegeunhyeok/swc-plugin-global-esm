const React = global.__modules.import("react").default;
const useState = global.__modules.import("react").useState;
const useEffect = global.__modules.import("react").useEffect;
const Container = global.__modules.import("@app/components").Container;
const Section = global.__modules.import("@app/components").Section;
const Button = global.__modules.import("@app/components").Button;
const Text = global.__modules.import("@app/components").Text;
const useCustomHook = global.__modules.import("@app/hooks").useCustomHook;
const app = global.__modules.import("@app/core");
function MyComponent() {
  return null;
}
const __export_default = class {
  init() {
    // empty
  }
};
global.__modules.export("test.js", {
  MyComponent,
  default: __export_default,
  app,
  useCustomHook
});
