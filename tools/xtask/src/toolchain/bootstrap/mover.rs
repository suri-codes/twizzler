use std::{path::PathBuf, process::Command};

use super::paths as bootstrap;
use crate::toolchain::pathfinding;

pub fn move_all(host_triple: &str, target_triple: &str) -> anyhow::Result<()> {
    let move_dir = |prev: PathBuf, next: PathBuf| -> anyhow::Result<()> {
        println!("moving {} to {}", prev.display(), next.display());

        // Remove destination if it exists
        if next.exists() {
            let _ = std::fs::remove_dir_all(&next);
        }

        if let Some(parent) = next.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let status = Command::new("cp")
            .arg("-r")
            .arg(&prev)
            .arg(&next)
            .status()?; // Use status() instead of spawn() to wait for completion

        if !status.success() {
            anyhow::bail!("cp command failed with status: {}", status);
        }

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
    let new_rust_stage_2 = pathfinding::get_rust_stage2_std(host_triple, target_triple)?;
    move_dir(old_rust_stage_2, new_rust_stage_2)?;

    // rust lld
    let old_rust_lld = bootstrap::get_rust_lld(host_triple)?;
    let new_rust_lld = pathfinding::get_rust_lld(host_triple)?;
    move_dir(old_rust_lld, new_rust_lld)?;

    //compiler_rt
    let old_compiler_rt = bootstrap::get_compiler_rt_path()?;
    let new_compiler_rt = pathfinding::get_compiler_rt_path()?;
    move_dir(old_compiler_rt, new_compiler_rt)?;

    // llvm bin
    // let old_llvm_bin = bootstrap::get_llvm_bin(host_triple)?;
    // let new_llvm_bin = pathfinding::get_llvm_bin(host_triple)?;
    // move_dir(old_llvm_bin, new_llvm_bin)?;

    // lld bin
    let old_lld_bin = bootstrap::get_lld_bin(host_triple)?;
    let new_lld_bin = pathfinding::get_lld_bin(host_triple)?;
    move_dir(old_lld_bin, new_lld_bin)?;

    Ok(())
}
