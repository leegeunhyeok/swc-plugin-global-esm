use std::collections::HashMap;

use super::GlobalEsmModule;
use swc_core::ecma::{
    transforms::testing::test,
    visit::{as_folder, Folder},
};

fn plugin(with_import_paths: bool) -> Folder<GlobalEsmModule> {
    let mut import_paths = HashMap::new();
    if with_import_paths {
        import_paths.insert(String::from("react"), String::from("node_modules/react/cjs/react.development.js"));
    }

    as_folder(GlobalEsmModule {
        module_name: String::from("test.js"),
        runtime_module: true,
        import_paths: Some(import_paths),
    })
}

test!(
    Default::default(),
    |_| plugin(false),
    without_import_paths,
    // Input codes
    r#"
    import React from 'react';
    "#,
    // Output codes after transformed with plugin
    r#"
    var React = global.__modules.import("react").default;
    global.__modules.export("test.js", null);
    "#
);

test!(
    Default::default(),
    |_| plugin(true),
    with_import_paths,
    // Input codes
    r#"
    import React from 'react';
    "#,
    // Output codes after transformed with plugin
    r#"
    var React = global.__modules.import("node_modules/react/cjs/react.development.js").default;
    global.__modules.export("test.js", null);
    "#
);
