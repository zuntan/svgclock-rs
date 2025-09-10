# ubuntu

```
sudo apt install build-essential curl git libgtk-3-dev
```

# Rust setup

- https://www.rust-lang.org/tools/install

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

```
rustup update
```

```
cargo install cargo-deb
```

- for build windows binary

```
rustup target list | grep windows

rustup target add x86_64-pc-windows-gnu
```

# git clone

```
git clone https://github.com/zuntan/svgclock-rs.git
```

# build

```
cd svgclock-rs
cargo build --release
ls target/release/svgclock-rs
cargo deb
ls target/debian/svgclock-rs_0.1.0-1_amd64.deb
```

- for build windows binary

```
sh build_x86_64-pc-windows-gnu.sh
ls svgclock-rs.zip
```