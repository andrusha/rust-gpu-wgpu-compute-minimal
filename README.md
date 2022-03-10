# Minimal Compute Shader Project

Uses [rust-gpu](https://github.com/EmbarkStudios/rust-gpu/) to compile shader into SPIR-V, which is then submitted by the [wgpu](https://github.com/gfx-rs/wgpu) backend running in [Vulkan
](https://www.vulkan.org/) compatibility mode. Requires [MoltenVK](https://moltengl.com/moltenvk/) for compatibility with [Metal](https://en.wikipedia.org/wiki/Metal_(API)) on macOs.

See also [pema99/rust-gpu-compute-example](https://github.com/pema99/rust-gpu-compute-example) and [rust-gpu/examples](https://github.com/EmbarkStudios/rust-gpu/tree/main/examples).

## Notes

- Each shader is defined as a bin crate and is compiled separately, see [build.rs](./build.rs) using [spirv-builder](https://github.com/EmbarkStudios/rust-gpu/tree/main/crates/spirv-builder)