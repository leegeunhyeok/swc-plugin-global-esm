use super::GlobalEsmModule;
use swc_core::ecma::{
    transforms::testing::test,
    visit::{as_folder, Folder},
};

fn plugin() -> Folder<GlobalEsmModule> {
    as_folder(GlobalEsmModule {
        module_name: String::from("test.js"),
        runtime_module: true,
        import_paths: None,
    })
}

test!(
    Default::default(),
    |_| plugin(),
    export_named_var_decl,
    // Input codes
    r#"
    export const named = new Instance();
    "#,
    // Output codes after transformed with plugin
    r#"
    const named = new Instance();
    global.__modules.export("test.js", { named });
    "#
);

test!(
    Default::default(),
    |_| plugin(),
    export_named_fn_decl,
    // Input codes
    r#"
    export function namedFunction() {
        console.log('body');
    }
    "#,
    // Output codes after transformed with plugin
    r#"
    function namedFunction() {
        console.log('body');
    }
    global.__modules.export("test.js", { namedFunction });
    "#
);

test!(
    Default::default(),
    |_| plugin(),
    export_named,
    // Input codes
    r#"
    const plain = 0;
    const beforeRename = 1;
    export { plain, beforeRename as afterRename };
    "#,
    // Output codes after transformed with plugin
    r#"
    const plain = 0;
    const beforeRename = 1;
    global.__modules.export("test.js", { plain, afterRename: beforeRename });
    "#
);

test!(
    Default::default(),
    |_| plugin(),
    export_named_with_alias,
    // Input codes
    r#"
    export * as rename from 'module';
    "#,
    // Output codes after transformed with plugin
    r#"
    const __export_named = global.__modules.import("module");
    global.__modules.export("test.js", { rename: __export_named });
    "#
);

test!(
    Default::default(),
    |_| plugin(),
    export_default_expr,
    // Input codes
    r#"
    export default 0;
    "#,
    // Output codes after transformed with plugin
    r#"
    const __export_default = 0;
    global.__modules.export("test.js", {
        default: __export_default
    });
    "#
);

test!(
    Default::default(),
    |_| plugin(),
    export_default_decl,
    // Input codes
    r#"
    export default class ClassDecl {}
    "#,
    // Output codes after transformed with plugin
    r#"
    class ClassDecl {}
    global.__modules.export("test.js", {
        default: ClassDecl
    });
    "#
);

test!(
    Default::default(),
    |_| plugin(),
    export_default_decl_anonymous,
    // Input codes
    r#"
    export default class {}
    "#,
    // Output codes after transformed with plugin
    r#"
    const __export_default = class {}
    global.__modules.export("test.js", {
        default: __export_default
    });
    "#
);

test!(
    Default::default(),
    |_| plugin(),
    export_all,
    // Input codes
    r#"
    export * from 'module';
    "#,
    // Output codes after transformed with plugin
    r#"
    const __export_all = global.__modules.import("module");
    global.__modules.export("test.js", { ...__export_all });
    "#
);

test!(
    Default::default(),
    |_| plugin(),
    export_all_partial,
    // Input codes
    r#"
    export { a, b, c } from 'module';
    "#,
    // Output codes after transformed with plugin
    r#"
    const a = global.__modules.import("module").a;
    const b = global.__modules.import("module").b;
    const c = global.__modules.import("module").c;
    global.__modules.export("test.js", { a, b, c });
    "#
);

test!(
    Default::default(),
    |_| plugin(),
    non_exports,
    // Input codes
    r#"
    import __dummy from 'dummy';
    "#,
    // Output codes after transformed with plugin
    r#"
    const __dummy = global.__modules.import("dummy").default;
    global.__modules.export("test.js", null);
    "#
);

test!(
    Default::default(),
    |_| plugin(),
    export_mixed,
    // Input codes
    r#"
    import React, { useState, useEffect } from 'react';
    import { Container, Section, Button, Text } from '@app/components';
    import { useCustomHook } from '@app/hooks';
    import * as app from '@app/core';

    export function MyComponent () {
        return null;
    }

    export default class {
        init() {
            // empty
        }
    }

    export { app, useCustomHook };
    "#,
    // Output codes after transformed with plugin
    r#"
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
        init() {}
    };
    global.__modules.export("test.js", {
        MyComponent,
        default: __export_default,
        app,
        useCustomHook
    });
    "#
);
