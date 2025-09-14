[![License: Unlicense](https://img.shields.io/badge/License-Unlicense-blue?style=flat-square)](Unlicense)

## N64 Project Template for Rust GCC

Base for building Nintendo 64 projects written in Rust using GCC as a compiler
backend.

## Setup

1. Install [Rust](https://www.rust-lang.org/tools/install).

1. Clone this repository.
   ```sh
   git clone https://github.com/LateGator/rust-n64-gcc-template.git
   cd rust-n64-gcc-template
   ```

1. Install the nust64 runner.
   ```sh
   cargo install nust64
   ```

1. Set the `N64_INST` environment variable to where you would like the
   toolchain to be installed.
   ```sh
   export N64_INST=/opt/rust64
   ```

1. Build and install the toolchain. This will take a while.
   ```sh
   ./toolchain.sh install all
   ```
   The `n64-toolchain` folder will be used as a work directory for the
   toolchain sources and build data.

## Targets

A selection of target JSON files will be installed in the `.cargo` folder for
each supported MIPS ABI.

- `mips64vr-n64-elfeabi64` (Default)
- `mips64vr-n64-elfeabi32`
- `mips64vr-n64-elfn32`
- `mips64vr-n64-elfo64`
- `mips64vr-n64-elfo32`

A different ABI can be selected by editing `.cargo/config.toml`. Additional GCC
arguments can be specified using the rustc `-Cllvm-args=...` switch specified
in the target `rustflags` field at the bottom.

## Building

```sh
cargo r                         # Builds debug ROM
cargo r --release               # Builds release ROM
cargo r --profile dev-opt       # Builds optimized debug ROM
```
