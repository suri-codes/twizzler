use std::{path::PathBuf, process::Command};

use cargo::try_old_curl_http2_pipewait;

use super::pathfinding;

pub fn move_all(host_triple: &str, target_triple: &str) -> anyhow::Result<()> {
    let install_dir = std::env::current_dir()
        .unwrap()
        .join("./toolchain/install/");

    let move_dir = |prev: PathBuf, next: PathBuf| -> anyhow::Result<()> {
        let _ = Command::new("mv")
            .arg(prev.to_str().unwrap())
            .arg(next.to_str().unwrap())
            .spawn()?;
        Ok(())
    };

    // llvm native runtime
    let old_llvm_rt = get_llvm_native_runtime(target_triple)?;
    let new_llvm_rt = pathfinding::get_llvm_native_runtime_install(target_triple)?;
    move_dir(old_llvm_rt, new_llvm_rt)?;

    // llvm native runtime install
    let old_llvm_native_install = get_llvm_native_runtime(target_triple)?;
    let new_llvm_native_install = pathfinding::get_llvm_native_runtime_install(target_triple)?;
    move_dir(old_llvm_native_install, new_llvm_native_install)?;

    // rust stage2 std
    let old_rust_stage_2 = get_rust_stage2_std(host_triple, target_triple)?;
    let new_rust_stage_2 = pathfinding::get_llvm_native_runtime_install(target_triple)?;
    move_dir(old_rust_stage_2, new_rust_stage_2)?;

    // rust lld
    let old_rust_lld = get_rust_lld(host_triple)?;
    let new_rust_lld = pathfinding::get_rust_lld(host_triple)?;
    move_dir(old_rust_lld, new_rust_lld)?;

    // llvm bin
    let old_llvm_bin = get_llvm_bin(host_triple)?;
    let new_llvm_bin = pathfinding::get_llvm_bin(host_triple)?;
    move_dir(old_llvm_bin, new_llvm_bin)?;

    // lld bin
    let old_lld_bin = get_lld_bin(host_triple)?;
    let new_lld_bin = pathfinding::get_lld_bin(host_triple)?;
    move_dir(old_lld_bin, new_lld_bin)?;

    Ok(())
}

fn get_rust_stage2_std(host_triple: &str, target_triple: &str) -> anyhow::Result<PathBuf> {
    let curdir = std::env::current_dir().unwrap();
    let dir = curdir
        //TODO: move this into install
        .join("toolchain/src/rust/build")
        .join(host_triple)
        .join("stage2-std")
        .join(target_triple)
        .join("release");
    Ok(dir)
}

fn get_llvm_native_runtime(target_triple: &str) -> anyhow::Result<PathBuf> {
    let curdir = std::env::current_dir().unwrap();
    let arch = target_triple.split("-").next().unwrap();
    let archive_name = format!("libclang_rt.builtins-{}.a", arch);
    //TODO: move this into install
    let dir = curdir
        .join("toolchain/src/rust/build")
        .join(target_triple)
        .join("native/sanitizers/build/lib/twizzler")
        .join(archive_name);
    Ok(dir)
}
fn get_rust_lld(host_triple: &str) -> anyhow::Result<PathBuf> {
    let curdir = std::env::current_dir().unwrap();
    let rustlib_bin = curdir
        //TODO: move this into install
        .join("toolchain/src/rust/build")
        .join(host_triple)
        .join("stage1/lib/rustlib")
        .join(host_triple)
        .join("bin/rust-lld");
    Ok(rustlib_bin)
}
fn get_llvm_bin(host_triple: &str) -> anyhow::Result<PathBuf> {
    let curdir = std::env::current_dir().unwrap();
    let llvm_bin = curdir
        //TODO: move this into install
        .join("toolchain/src/rust/build")
        .join(host_triple)
        .join("llvm/bin");
    Ok(llvm_bin)
}

fn get_lld_bin(host_triple: &str) -> anyhow::Result<PathBuf> {
    let curdir = std::env::current_dir().unwrap();
    let llvm_bin = curdir
        //TODO: move this into install
        .join("toolchain/src/rust/build")
        .join(host_triple)
        .join("lld/bin");
    Ok(llvm_bin)
}
