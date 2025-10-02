// The spirv tools use generated code, for now we just replicate the minimum
// generation we need here by calling the *shudders* python script(s) we need
// to in a simple script and commit them to source control, as they only need
// to be regenerated when spirv-headers is updated

use std::path::PathBuf;
use std::{fs, process::Command};

fn python<S: AsRef<std::ffi::OsStr>>(args: impl IntoIterator<Item = S>) -> Result<(), i32> {
    Command::new("python")
        .args(args)
        .status()
        .map_err(|_e| -1)
        .and_then(|es| {
            if es.success() {
                Ok(())
            } else {
                Err(es.code().unwrap_or(-1))
            }
        })
}

fn main() {
    let sys_root = PathBuf::from_iter([env!("CARGO_MANIFEST_DIR"), "..", "..", "spirv-tools-sys"]);
    fs::create_dir_all(sys_root.join("generated")).expect("unable to create 'generated'");
    std::env::set_current_dir(&sys_root).unwrap();

    python([
        "spirv-tools/utils/update_build_version.py",
        "spirv-tools/CHANGES",
        "generated/build-version.inc",
    ])
    .expect("failed to generate build version from spirv-headers");

    grammar_tables();

    // This will eventually be moved to spirv-headers
    generate_header(
        "NonSemanticShaderDebugInfo100",
        "nonsemantic.shader.debuginfo.100",
    );

    registry_table();
}

const GRAMMAR_DIR: &str = "spirv-headers/include/spirv/unified1";

fn grammar_tables() {
    python(&[
        "spirv-tools/utils/ggt.py".to_owned(),
        "--core-tables-body-output=generated/core_tables_body.inc".into(),
		"--core-tables-header-output=generated/core_tables_header.inc".into(),
		format!("--spirv-core-grammar={GRAMMAR_DIR}/spirv.core.grammar.json"),
		format!("--extinst=,{GRAMMAR_DIR}/extinst.debuginfo.grammar.json"),
		format!("--extinst=,{GRAMMAR_DIR}/extinst.glsl.std.450.grammar.json"),
		format!("--extinst=,{GRAMMAR_DIR}/extinst.nonsemantic.clspvreflection.grammar.json"),
		format!("--extinst=SHDEBUG100_,{GRAMMAR_DIR}/extinst.nonsemantic.shader.debuginfo.100.grammar.json"),
		format!("--extinst=,{GRAMMAR_DIR}/extinst.nonsemantic.vkspreflection.grammar.json"),
		format!("--extinst=CLDEBUG100_,{GRAMMAR_DIR}/extinst.opencl.debuginfo.100.grammar.json"),
		format!("--extinst=,{GRAMMAR_DIR}/extinst.opencl.std.100.grammar.json"),
		format!("--extinst=,{GRAMMAR_DIR}/extinst.spv-amd-gcn-shader.grammar.json"),
		format!("--extinst=,{GRAMMAR_DIR}/extinst.spv-amd-shader-ballot.grammar.json"),
		format!("--extinst=,{GRAMMAR_DIR}/extinst.spv-amd-shader-explicit-vertex-parameter.grammar.json"),
		format!("--extinst=,{GRAMMAR_DIR}/extinst.spv-amd-shader-trinary-minmax.grammar.json"),
    ]).expect("failed to generate grammar tables from spirv-headers");
}

fn registry_table() {
    python([
        "spirv-tools/utils/generate_registry_tables.py",
        "--xml=spirv-headers/include/spirv/spir-v.xml",
        "--generator=generated/generators.inc",
    ])
    .expect("failed to generate core table from spirv-headers");
}

fn generate_header(header_name: &str, grammar: &str) {
    python(&[
        "spirv-tools/utils/generate_language_headers.py".to_owned(),
        format!("--extinst-grammar={GRAMMAR_DIR}/extinst.{grammar}.grammar.json",),
        format!("--extinst-output-path=generated/{}.h", header_name),
    ])
    .expect("failed to generate C header");
}
