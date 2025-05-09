#![allow(clippy::unnecessary_wraps)]

const SPIRV_BIN: &[u8] = include_bytes!("wgpu_example_shader.spv");

fn validate_compiled(_input: &[u32]) -> Option<Result<(), spirv_tools::Error>> {
    #[cfg(feature = "use-compiled-tools")]
    {
        use spirv_tools::val::{compiled::CompiledValidator, Validator};
        let cv = CompiledValidator::default();
        Some(cv.validate(_input, None))
    }
    #[cfg(not(feature = "use-compiled-tools"))]
    None
}

fn validate_tool(_input: &[u32]) -> Option<Result<(), spirv_tools::Error>> {
    #[cfg(feature = "use-installed-tools")]
    {
        use spirv_tools::val::{tool::ToolValidator, Validator};
        let cv = ToolValidator::default();
        Some(cv.validate(_input, None))
    }
    #[cfg(not(feature = "use-installed-tools"))]
    None
}

#[test]
fn gets_error_message() {
    let expected_msg = "error:0:0 - Loop header '6[%loop_header]' is targeted by 2 back-edge blocks but the standard requires exactly one";
    let expected_notes = "  %loop_header = OpLabel\n";

    let spirv = SPIRV_BIN
        .chunks_exact(size_of::<u32>())
        .map(|chunk| u32::from_ne_bytes(chunk.try_into().unwrap()))
        .collect::<Vec<_>>();

    for res in validate_compiled(&spirv)
        .into_iter()
        .chain(validate_tool(&spirv).into_iter())
    {
        let err = res.unwrap_err();
        let diag = err.diagnostic.as_ref().unwrap();
        assert_eq!(diag.line, 0);
        assert_eq!(diag.column, 0);
        assert_eq!(diag.message, &expected_msg[12..]);
        assert_eq!(diag.notes, expected_notes);

        let err_str = err.to_string();
        assert_eq!(&err_str[..113], expected_msg);
        assert_eq!(&err_str[113 + 1..], expected_notes);
    }
}
