#!/bin/bash

# $ sudo apt install gcc-mingw-w64-x86-64-win32
# $ rustup target add aarch64-unknown-linux-gnu
# $ rustup target list --installed

GTK3_LIB=/opt2/_dev/GTK3_Gvsbuild_2025.8.0_x64
export PKG_CONFIG_LIBDIR=$GTK3_LIB/lib/pkgconfig
export PKG_CONFIG_SYSROOT_DIR_x86_64_pc_windows_gnu=$GTK3_LIB
export RUSTFLAGS="-L $GTK3_LIB/lib -C link-arg=-Wl,-subsystem,windows"

cargo clean           --target x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu