use std::path::{Path, PathBuf};

use super::generate_tag;
use crate::triple::Triple;

//TODO: this should return the canonicalized path to the toolchain!
pub fn get_toolchain_path() -> anyhow::Result<PathBuf> {
    let mut curr_dir = std::env::current_dir()?;
    curr_dir.push("toolchain/install");
    // Ok(curr_dir.to_str().unwrap().to_owned())

    let tag = generate_tag()?;
    curr_dir.push("toolchain");
    curr_dir.push(tag);

    Ok(curr_dir)
    // Ok(curr_dir.to_str().unwrap().to_owned())
}

pub fn get_rustc_path() -> anyhow::Result<PathBuf> {
    let mut rustc_path = get_toolchain_path()?;

    rustc_path.push("bin/rustc");

    Ok(rustc_path)
    // Ok(format!("{}/bin/rustc", rustc_path))
}

pub fn get_rustdoc_path() -> anyhow::Result<PathBuf> {
    let mut rustdoc_path = get_toolchain_path()?;

    rustdoc_path.push("/bin/rustdoc");
    Ok(rustdoc_path)

    // Ok(format!("{}/bin/rustdoc", toolchain))
}

pub fn get_bin_path() -> anyhow::Result<PathBuf> {
    let mut toolchain_bins = get_toolchain_path()?;
    toolchain_bins.push("bin");
    Ok(toolchain_bins)

    // Ok(format!("{}/bin", toolchain_bins.to_string_lossy()))
}

pub fn set_cc(target: &Triple) -> anyhow::Result<()> {
    let toolchain_path = get_toolchain_path()?;

    let clang_path = {
        let mut clang_path = toolchain_path.clone();
        clang_path.push("bin/clang");
        clang_path.canonicalize().unwrap()
    };

    // When compiling crates that compile C code (e.g. alloca), we need to use our clang.
    // let clang_path = Path::new(format!("{}/bin/clang", toolchain_path).as_str())
    //     .canonicalize()
    //     .unwrap();
    std::env::set_var("CC", &clang_path);
    std::env::set_var("LD", &clang_path);
    std::env::set_var("CXX", &clang_path);

    // We don't have any real system-include files, but we can provide these extremely simple ones.
    let sysroot_path = Path::new(&format!(
        "{}/sysroots/{}",
        toolchain_path.to_string_lossy(),
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

    Ok(())
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
    let llvm_bin = PathBuf::from(get_toolchain_path()?)
        .join("rust/build")
        .join(host_triple)
        .join("lld/bin");
    Ok(llvm_bin)
}

pub fn get_llvm_bin(host_triple: &str) -> anyhow::Result<PathBuf> {
    let llvm_bin = PathBuf::from(get_toolchain_path()?)
        .join("rust/build")
        .join(host_triple)
        .join("llvm/bin");
    Ok(llvm_bin)
}

pub fn get_rustlib_bin(host_triple: &str) -> anyhow::Result<PathBuf> {
    let rustlib_bin = PathBuf::from(get_toolchain_path()?)
        .join("toolchain/install/lib/rustlib")
        .join(host_triple)
        .join("bin");
    Ok(rustlib_bin)
}

pub fn get_rustlib_lib(host_triple: &str) -> anyhow::Result<PathBuf> {
    let rustlib_bin = PathBuf::from(get_toolchain_path()?)
        .join("lib/rustlib")
        .join(host_triple)
        .join("lib");
    Ok(rustlib_bin)
}

pub fn get_compiler_rt_path() -> anyhow::Result<PathBuf> {
    let compiler_rt =
        PathBuf::from(get_toolchain_path()?).join("rust/src/llvm-project/compiler-rt");

    Ok(compiler_rt)
}

pub fn get_rust_lld(host_triple: &str) -> anyhow::Result<PathBuf> {
    let rustlib_bin = PathBuf::from(get_toolchain_path()?)
        .join("rust/build")
        .join(host_triple)
        .join("stage1/lib/rustlib")
        .join(host_triple)
        .join("bin/rust-lld");
    Ok(rustlib_bin)
}

pub fn get_rust_stage2_std(host_triple: &str, target_triple: &str) -> anyhow::Result<PathBuf> {
    let dir = PathBuf::from(get_toolchain_path()?)
        .join("rust/build")
        .join(host_triple)
        .join("stage2-std")
        .join(target_triple)
        .join("release");

    Ok(dir)
}

pub fn get_llvm_native_runtime(target_triple: &str) -> anyhow::Result<PathBuf> {
    let arch = target_triple.split("-").next().unwrap();
    let archive_name = format!("libclang_rt.builtins-{}.a", arch);

    let dir = PathBuf::from(get_toolchain_path()?)
        .join("rust/build")
        .join(target_triple)
        .join("native/sanitizers/build/lib/twizzler")
        .join(archive_name);
    Ok(dir)
}

pub fn get_llvm_native_runtime_install(target_triple: &str) -> anyhow::Result<PathBuf> {
    let archive_name = "libclang_rt.builtins.a";
    let dir = PathBuf::from(get_toolchain_path()?)
        .join("lib/clang/20/lib")
        .join(target_triple)
        .join(archive_name);
    Ok(dir)
}

pub fn get_builtin_headers() -> anyhow::Result<PathBuf> {
    let headers = PathBuf::from(get_toolchain_path()?).join("lib/clang/20/include/");

    Ok(headers)
}
