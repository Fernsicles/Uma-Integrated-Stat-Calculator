#!/usr/bin/env bash

sed -i 's|/DEF:||' 'Hachimi-Edge/build.rs'
sed -i '/crate-type = \["cdylib"\]/d' 'Hachimi-Edge/Cargo.toml'
cargo build --release --target x86_64-pc-windows-gnu
