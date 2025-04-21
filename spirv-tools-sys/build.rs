use cc::Build;
use std::path::Path;

fn add_includes(builder: &mut Build, root: &str, includes: &[&str]) {
    let root = Path::new(root);

    for inc in includes {
        builder.include(root.join(inc));
    }
}

fn add_sources(builder: &mut Build, root: &str, files: &[&str]) {
    let root = Path::new(root);
    builder.files(files.iter().map(|src| {
        let mut p = root.join(src);
        p.set_extension("cpp");
        p
    }));
}

fn add_sources_glob(builder: &mut Build, root: &str) {
    let root = Path::new(root);
    builder.files(root.read_dir().expect("root is a dir").filter_map(|a| {
        let entry = a.unwrap();
        let is_cpp = entry.file_name().to_str().unwrap().ends_with(".cpp");
        is_cpp.then_some(entry.path())
    }));
}

fn shared(build: &mut Build) {
    add_sources(
        build,
        "spirv-tools/source",
        &[
            "util/bit_vector",
            "util/parse_number",
            "util/string_utils",
            "assembly_grammar",
            "binary",
            "diagnostic",
            "disassemble",
            "enum_string_mapping",
            "ext_inst",
            "extensions",
            "libspirv",
            "name_mapper",
            "opcode",
            "operand",
            "parsed_operand",
            "print",
            "software_version",
            "spirv_endian",
            "spirv_fuzzer_options",
            "spirv_optimizer_options",
            "spirv_reducer_options",
            "spirv_target_env",
            "spirv_validator_options",
            "table",
            "text",
            "text_handler",
            "to_string",
        ],
    );
}

fn opt(build: &mut Build) {
    build.file("src/c/opt.cpp");

    add_sources_glob(build, "spirv-tools/source/opt");
}

fn val(build: &mut Build) {
    add_sources_glob(build, "spirv-tools/source/val");
}

fn main() {
    let use_installed = std::env::var("CARGO_FEATURE_USE_INSTALLED_TOOLS").is_ok();
    let use_compiled = std::env::var("CARGO_FEATURE_USE_COMPILED_TOOLS").is_ok();

    if !use_compiled && !use_installed {
        panic!("Enable at least one of `use-compiled-tools` or `use-installed-tools` features");
    }

    if use_installed && !use_compiled {
        println!("cargo:warning=use-installed-tools feature on, skipping compilation of C++ code");
        return;
    }

    let mut build = Build::new();

    add_includes(&mut build, "spirv-tools", &["", "include"]);
    add_includes(&mut build, "generated", &[""]);
    add_includes(
        &mut build,
        "spirv-headers",
        &["include", "include/spirv/unified1"],
    );

    shared(&mut build);
    val(&mut build);
    opt(&mut build);

    build.define("SPIRV_CHECK_CONTEXT", None);

    let target_def = match std::env::var("CARGO_CFG_TARGET_OS")
        .expect("CARGO_CFG_TARGET_OS not set")
        .as_str()
    {
        "linux" => "SPIRV_LINUX",
        "windows" => "SPIRV_WINDOWS",
        "macos" => "SPIRV_MAC",
        android if android.starts_with("android") => "SPIRV_ANDROID",
        "freebsd" => "SPIRV_FREEBSD",
        other => panic!("unsupported target os '{other}'"),
    };

    build.define(target_def, None);

    let compiler = build.get_compiler();

    if compiler.is_like_gnu() {
        build
            .flag("-Wall")
            .flag("-Wextra")
            .flag("-Wnon-virtual-dtor")
            .flag("-Wno-missing-field-initializers")
            .flag("-Werror")
            .flag("-std=c++17")
            .flag("-fno-exceptions")
            .flag("-fno-rtti")
            .flag("-Wno-long-long")
            .flag("-Wshadow")
            .flag("-Wundef")
            .flag("-Wconversion")
            .flag("-Wno-sign-conversion")
            .flag("-Wno-deprecated-declarations"); // suppress warnings about sprintf
    } else if compiler.is_like_clang() {
        build
            .flag("-Wextra-semi")
            .flag("-Wall")
            .flag("-Wextra")
            .flag("-Wnon-virtual-dtor")
            .flag("-Wno-missing-field-initializers")
            .flag("-Wno-self-assign")
            .flag("-Werror")
            .flag("-std=c++17")
            .flag("-fno-exceptions")
            .flag("-fno-rtti")
            .flag("-Wno-long-long")
            .flag("-Wshadow")
            .flag("-Wundef")
            .flag("-Wconversion")
            .flag("-Wno-sign-conversion")
            .flag("-Wno-deprecated-declarations") // suppress warnings about sprintf
            .flag("-ftemplate-depth=1024");
    } else if compiler.is_like_msvc() {
        build.flag("/std:c++17");
    }

    build.cpp(true);
    build.compile("spirv-tools");

    println!("cargo:rerun-if-changed=spirv-tools");
    println!("cargo:rerun-if-changed=spirv-headers");
}
