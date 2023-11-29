use std::{collections::HashMap, path::PathBuf};

use swc_ecma_parser::{Syntax, TsConfig};
use swc_ecma_transforms_testing::test_fixture;
use swc_global_esm::global_esm;

#[testing::fixture("tests/fixture/import/**/input.js")]
#[testing::fixture("tests/fixture/export/**/input.js")]
#[testing::fixture("tests/fixture/normalize_src/**/input.js")]
fn fixture(input: PathBuf) {
    let filename = input.to_string_lossy();
    let output = input.with_file_name("output.js");

    test_fixture(
        Syntax::Typescript(TsConfig {
            tsx: filename.ends_with(".tsx"),
            ..Default::default()
        }),
        &|_| global_esm(String::from("test.js"), true, None),
        &input,
        &output,
        Default::default(),
    );
}

#[testing::fixture("tests/fixture/non-runtime/**/input.js")]
fn fixture_non_runtime(input: PathBuf) {
    let filename = input.to_string_lossy();
    let output = input.with_file_name("output.js");

    test_fixture(
        Syntax::Typescript(TsConfig {
            tsx: filename.ends_with(".tsx"),
            ..Default::default()
        }),
        &|_| global_esm(String::from("test.js"), false, None),
        &input,
        &output,
        Default::default(),
    );
}

#[testing::fixture("tests/fixture/import_paths/input.js")]
fn fixture_import_paths(input: PathBuf) {
    let filename = input.to_string_lossy();
    let output = input.with_file_name("output.js");
    let import_paths = HashMap::from([(
        String::from("react"),
        String::from("node_modules/react/cjs/react.development.js"),
    )]);

    test_fixture(
        Syntax::Typescript(TsConfig {
            tsx: filename.ends_with(".tsx"),
            ..Default::default()
        }),
        &|_| global_esm(String::from("test.js"), true, Some(import_paths.to_owned())),
        &input,
        &output,
        Default::default(),
    );
}
