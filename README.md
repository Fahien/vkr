# VKR

VKR is a Vulkan experiment written in Rust to explore [ash](https://github.com/MaikKlein/ash) and [rust-gpu](https://github.com/EmbarkStudios/rust-gpu).

## Requirements

Download and install [Vulkan SDK 1.2.182.0](https://vulkan.lunarg.com/sdk/home) for your platform.

## Build

Run `script/build-vkr.sh`. This script will make sure you are using the right `rust-gpu` version, together with its `rust-toolchain`.
The script also makes sure that `cargo test` runs to ensure that required files are generated.

## Troubleshooting

If you see the following error message but your rustup version is `1.23` already, then uninstall and re-install it following the method suggested on `rustup.rs`.

```
If you see this, run `rustup self update` to get rustup 1.23 or newer.
```

---

Before attempting to fix any kind of Vulkan validation error, make sure your installed Vulkan SDK version is aligned with the requirements and the rust-gpu version is correct as from `.gitsubmodules` together with its `rust-toolchain` file.
