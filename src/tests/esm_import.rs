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
    default_import,
    // Input codes
    r#"
    import React from 'react';
    "#,
    // Output codes after transformed with plugin
    r#"
    const React = global.__modules.import("react").default;
    global.__modules.export("test.js", null);
    "#
);

test!(
    Default::default(),
    |_| plugin(),
    named_import,
    // Input codes
    r#"
    import { useState, useContext } from 'react';
    "#,
    // Output codes after transformed with plugin
    r#"
    const useState = global.__modules.import("react").useState;
    const useContext = global.__modules.import("react").useContext;
    global.__modules.export("test.js", null);
    "#
);

test!(
    Default::default(),
    |_| plugin(),
    mixed_import,
    // Input codes
    r#"
    import React, { useState, useContext } from 'react';
    "#,
    // Output codes after transformed with plugin
    r#"
    const React = global.__modules.import("react").default;
    const useState = global.__modules.import("react").useState;
    const useContext = global.__modules.import("react").useContext;
    global.__modules.export("test.js", null);
    "#
);

test!(
    Default::default(),
    |_| plugin(),
    import_all,
    // Input codes
    r#"
    import * as ReactAll from 'react';
    "#,
    // Output codes after transformed with plugin
    r#"
    const ReactAll = global.__modules.import("react");
    global.__modules.export("test.js", null);
    "#
);

test!(
    Default::default(),
    |_| plugin(),
    import_with_stmt,
    // Input codes
    r#"
    import React, { useState, useContext } from 'react';
    function testFn() {}
    class TestClass{}
    "#,
    // Output codes after transformed with plugin
    r#"
    const React = global.__modules.import("react").default;
    const useState = global.__modules.import("react").useState;
    const useContext = global.__modules.import("react").useContext;
    function testFn() {}
    class TestClass {}
    global.__modules.export("test.js", null);
    "#
);
