use std::{path::PathBuf, process::Command};

use super::paths as bootstrap;
use crate::toolchain::pathfinding;

pub fn move_all(host_triple: &str, target_triple: &str) -> anyhow::Result<()> {
    // let install_dir = std::env::current_dir()
    //     .unwrap()
    //     .join("./toolchain/install/");

    let move_dir = |prev: PathBuf, next: PathBuf| -> anyhow::Result<()> {
        let _ = Command::new("cp -r")
            .arg(prev.to_str().unwrap())
            .arg(next.to_str().unwrap())
            .spawn()?;
        Ok(())
    };

    // llvm native runtime
    let old_llvm_rt = bootstrap::get_llvm_native_runtime(target_triple)?;
    let new_llvm_rt = pathfinding::get_llvm_native_runtime_install(target_triple)?;
    move_dir(old_llvm_rt, new_llvm_rt)?;

    // llvm native runtime install
    let old_llvm_native_install = bootstrap::get_llvm_native_runtime(target_triple)?;
    let new_llvm_native_install = pathfinding::get_llvm_native_runtime_install(target_triple)?;
    move_dir(old_llvm_native_install, new_llvm_native_install)?;

    // rust stage2 std
    let old_rust_stage_2 = bootstrap::get_rust_stage2_std(host_triple, target_triple)?;
    let new_rust_stage_2 = pathfinding::get_llvm_native_runtime_install(target_triple)?;
    move_dir(old_rust_stage_2, new_rust_stage_2)?;

    // rust lld
    let old_rust_lld = bootstrap::get_rust_lld(host_triple)?;
    let new_rust_lld = pathfinding::get_rust_lld(host_triple)?;
    move_dir(old_rust_lld, new_rust_lld)?;

    // llvm bin
    let old_llvm_bin = bootstrap::get_llvm_bin(host_triple)?;
    let new_llvm_bin = pathfinding::get_llvm_bin(host_triple)?;
    move_dir(old_llvm_bin, new_llvm_bin)?;

    // lld bin
    let old_lld_bin = bootstrap::get_lld_bin(host_triple)?;
    let new_lld_bin = pathfinding::get_lld_bin(host_triple)?;
    move_dir(old_lld_bin, new_lld_bin)?;

    Ok(())
}
