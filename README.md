<div align="center">

# VKR 🌋

**VKR is a Vulkan experiment written in Rust to explore [ash](https://github.com/MaikKlein/ash) and [rust-gpu](https://github.com/EmbarkStudios/rust-gpu)**

![Cube](https://user-images.githubusercontent.com/6058008/129566096-8ed6d53a-50e7-43d8-9dce-7410bc02d64a.gif)

</div>

## Features

- Data-driven design
- Scene graph
- Rust shaders
- Swapchain recreation on window resize
- Concurrent frames drawing
- Various rendering pipelines
- Multiple subpasses

## Requirements

VKR should work on Linux, MacOS, and Windows without issues, but if you need any help please do not hesitate to contact me.

Download and install [Vulkan SDK 1.2.198.1](https://vulkan.lunarg.com/sdk/home) for your platform. Install [Rust](https://rustup.rs/) and [SDL2](https://www.libsdl.org/download-2.0.php).

## Build

Run `script/build-vkr.sh`. This script will make sure you are using the right `rust-gpu` version, together with its `rust-toolchain`.
The script also makes sure that `cargo test` runs to ensure that required files are generated.

## Troubleshooting

- If you see the following error message but your rustup version is `1.23` already, then uninstall and re-install it following the method suggested on `rustup.rs`.
  ```
  If you see this, run `rustup self update` to get rustup 1.23 or newer.
  ```

- Before attempting to fix any kind of Vulkan validation error, make sure your Vulkan SDK version installed is aligned with the requirements and the rust-gpu version is correct as from `.gitsubmodules` together with its `rust-toolchain` file.

## Screenshots

<div align="center">

<table>
<tr>
<td align="center">

![Simple start](https://user-images.githubusercontent.com/6058008/111353245-664c9780-8685-11eb-8cfb-8f16c1549326.gif)

</td>
<td align="center">

![Hello texture](https://user-images.githubusercontent.com/6058008/111924135-30ebe380-8aa3-11eb-9f5d-c668bdac0174.gif)

</td>
</tr>

<tr>
<td colspan="2" align="center">

![Parallax](https://user-images.githubusercontent.com/6058008/114322696-e6014100-9b21-11eb-84ac-43932d0a71c6.gif)

</td>
</tr>
</table>

</div>