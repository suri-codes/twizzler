use std::{
    fs::{remove_dir_all, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    vec,
};

use anyhow::Context;
use fs_extra::dir::CopyOptions;
use guess_host_triple::guess_host_triple;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use toml_edit::DocumentMut;

use crate::triple::{all_possible_platforms, Triple};

pub fn get_toolchain_path() -> anyhow::Result<String> {
    Ok("toolchain/install".to_string())
}

pub fn get_rustc_path() -> anyhow::Result<String> {
    let toolchain = get_toolchain_path()?;
    Ok(format!("{}/bin/rustc", toolchain))
}

pub fn get_rustdoc_path() -> anyhow::Result<String> {
    let toolchain = get_toolchain_path()?;
    Ok(format!("{}/bin/rustdoc", toolchain))
}

pub fn set_cc(target: &Triple) {
    // When compiling crates that compile C code (e.g. alloca), we need to use our clang.
    let clang_path = Path::new("toolchain/install/bin/clang")
        .canonicalize()
        .unwrap();
    std::env::set_var("CC", &clang_path);
    std::env::set_var("LD", &clang_path);
    std::env::set_var("CXX", &clang_path);

    // We don't have any real system-include files, but we can provide these extremely simple ones.
    let sysroot_path = Path::new(&format!(
        "toolchain/install/sysroots/{}",
        target.to_string()
    ))
    .canonicalize()
    .unwrap();
    // We don't yet support stack protector. Also, don't pull in standard lib includes, as those may
    // go to the system includes.
    let cflags = format!(
        "-fno-stack-protector -isysroot {} -target {} --sysroot {}",
        sysroot_path.display(),
        target.to_string(),
        sysroot_path.display(),
    );
    std::env::set_var("CFLAGS", &cflags);
    std::env::set_var("LDFLAGS", &cflags);
    std::env::set_var("CXXFLAGS", &cflags);
}

pub fn clear_cc() {
    std::env::remove_var("CC");
    std::env::remove_var("CXX");
    std::env::remove_var("LD");
    std::env::remove_var("CC");
    std::env::remove_var("CXXFLAGS");
    std::env::remove_var("CFLAGS");
    std::env::remove_var("LDFLAGS");
}

pub fn clear_rustflags() {
    std::env::remove_var("RUSTFLAGS");
    std::env::remove_var("CARGO_TARGET_DIR");
}

pub fn get_lld_bin(host_triple: &str) -> anyhow::Result<PathBuf> {
    let curdir = std::env::current_dir().unwrap();
    let llvm_bin = curdir
        .join("toolchain/src/rust/build")
        .join(host_triple)
        .join("lld/bin");
    Ok(llvm_bin)
}

pub fn get_llvm_bin(host_triple: &str) -> anyhow::Result<PathBuf> {
    let curdir = std::env::current_dir().unwrap();
    let llvm_bin = curdir
        .join("toolchain/src/rust/build")
        .join(host_triple)
        .join("llvm/bin");
    Ok(llvm_bin)
}

pub fn get_rustlib_bin(host_triple: &str) -> anyhow::Result<PathBuf> {
    let curdir = std::env::current_dir().unwrap();
    let rustlib_bin = curdir
        .join("toolchain/install/lib/rustlib")
        .join(host_triple)
        .join("bin");
    Ok(rustlib_bin)
}

pub fn get_rustlib_lib(host_triple: &str) -> anyhow::Result<PathBuf> {
    let curdir = std::env::current_dir().unwrap();
    let rustlib_bin = curdir
        .join("toolchain/install/lib/rustlib")
        .join(host_triple)
        .join("lib");
    Ok(rustlib_bin)
}

pub fn get_rust_lld(host_triple: &str) -> anyhow::Result<PathBuf> {
    let curdir = std::env::current_dir().unwrap();
    let rustlib_bin = curdir
        .join("toolchain/src/rust/build")
        .join(host_triple)
        .join("stage1/lib/rustlib")
        .join(host_triple)
        .join("bin/rust-lld");
    Ok(rustlib_bin)
}

pub fn get_rust_stage2_std(host_triple: &str, target_triple: &str) -> anyhow::Result<PathBuf> {
    let curdir = std::env::current_dir().unwrap();
    let dir = curdir
        .join("toolchain/src/rust/build")
        .join(host_triple)
        .join("stage2-std")
        .join(target_triple)
        .join("release");
    Ok(dir)
}

pub fn get_llvm_native_runtime(target_triple: &str) -> anyhow::Result<PathBuf> {
    let curdir = std::env::current_dir().unwrap();
    let arch = target_triple.split("-").next().unwrap();
    let archive_name = format!("libclang_rt.builtins-{}.a", arch);
    let dir = curdir
        .join("toolchain/src/rust/build")
        .join(target_triple)
        .join("native/sanitizers/build/lib/twizzler")
        .join(archive_name);
    Ok(dir)
}

pub fn get_llvm_native_runtime_install(target_triple: &str) -> anyhow::Result<PathBuf> {
    let curdir = std::env::current_dir().unwrap();
    let archive_name = "libclang_rt.builtins.a";
    let dir = curdir
        .join("toolchain/install/lib/clang/20/lib")
        .join(target_triple)
        .join(archive_name);
    Ok(dir)
}
