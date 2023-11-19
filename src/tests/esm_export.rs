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
    global.__modules.export("test.js", { "named": named });
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
    global.__modules.export("test.js", { "namedFunction": namedFunction });
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
    global.__modules.export("test.js", {
        "plain": plain,
        "afterRename": beforeRename
    });
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
    var __export_named = global.__modules.import("module");
    global.__modules.export("test.js", { "rename": __export_named });
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
    var __export_default = 0;
    global.__modules.export("test.js", {
        "default": __export_default
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
        "default": ClassDecl
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
    var __export_default = class {}
    global.__modules.export("test.js", {
        "default": __export_default
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
    var __export_all = global.__modules.import("module");
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
    var a = global.__modules.import("module").a;
    var b = global.__modules.import("module").b;
    var c = global.__modules.import("module").c;
    global.__modules.export("test.js", {
        "a": a,
        "b": b,
        "c": c
    });
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
    var __dummy = global.__modules.import("dummy").default;
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
    var React = global.__modules.import("react").default;
    var useState = global.__modules.import("react").useState;
    var useEffect = global.__modules.import("react").useEffect;
    var Container = global.__modules.import("@app/components").Container;
    var Section = global.__modules.import("@app/components").Section;
    var Button = global.__modules.import("@app/components").Button;
    var Text = global.__modules.import("@app/components").Text;
    var useCustomHook = global.__modules.import("@app/hooks").useCustomHook;
    var app = global.__modules.import("@app/core");
    function MyComponent() {
        return null;
    }
    var __export_default = class {
        init() {}
    };
    global.__modules.export("test.js", {
        "MyComponent": MyComponent,
        "default": __export_default,
        "app": app,
        "useCustomHook": useCustomHook
    });
    "#
);
