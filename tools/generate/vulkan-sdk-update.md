# Vulkan sdk update

1. in `.gitmodules` set branch for both submodules to the new vulkan sdk version's branch
2. update submodules: `git submodule update --remote`
3. regenerate generated header files, requires python: `cargo run generate`
4. try to compile and run the tests, fixup any errors: `cargo test --all-features`
