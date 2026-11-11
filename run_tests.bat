@echo off
set PATH=C:\rust\mingw\bin;C:\rust\toolchain\bin;%PATH%
set RUSTUP_HOME=C:\rust\rustup
set CARGO_HOME=C:\rust\cargo
set CARGO_TARGET_DIR=C:\rust_target
set LIBRARY_PATH=C:\rust_target\lib
set RUSTFLAGS=-L C:/rust_target/lib
set RUSTC=C:\rust\toolchain\bin\rustc.exe
cd /d "%~dp0tauri_app\src-tauri"
C:\rust\toolchain\bin\cargo.exe test --bin cecdesk
