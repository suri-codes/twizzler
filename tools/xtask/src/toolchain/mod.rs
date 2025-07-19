use std::{
    fs::{remove_dir_all, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    vec,
};

use anyhow::Context;
use bootstrap::do_bootstrap;
use clap::Subcommand;
use fs_extra::dir::CopyOptions;
use guess_host_triple::guess_host_triple;
use indicatif::{ProgressBar, ProgressStyle};
use pathfinding::{
    get_lld_bin, get_llvm_bin, get_llvm_native_runtime, get_llvm_native_runtime_install,
    get_rust_lld, get_rustc_path, get_rustdoc_path, get_rustlib_bin,
};
use reqwest::Client;
use toml_edit::DocumentMut;
use utils::{download_file, install_build_tools};

use crate::triple::{all_possible_platforms, Triple};

mod bootstrap;
mod pathfinding;
mod utils;

pub use pathfinding::*;
pub use utils::*;

#[derive(clap::Args, Debug)]
pub struct BootstrapOptions {
    #[clap(long, help = "Skip downloading boot files from file server.")]
    skip_downloads: bool,
    #[clap(long, help = "Skip compiling the rust toolchain (not recommended...).")]
    skip_rust: bool,
    #[clap(
        long,
        help = "Don't remove the target/ directory after rebuilding the toolchain."
    )]
    keep_old_artifacts: bool,
    #[clap(
        long,
        help = "Keep early stages (0 and 1) of building rustc. Speeds up compilation, but can only be used if you (a) have already done a full bootstrap, and (b) since that bootstrap, all that is modified is twizzler-runtime-api or rust's standard library. Any changes to the compiler require one to not use this flag."
    )]
    keep_early_stages: bool,

    #[clap(long, help = "Skips pruning the toolchain after building")]
    skip_prune: bool,

    #[clap(
        long,
        help = "After bootstrapping, will compress and tag the toolchain for distribution."
    )]
    package: bool,

    #[clap(
        long,
        help = "Compresses the toolchain after bootstrapping for distribution"
    )]
    compress: bool,
}

#[derive(Subcommand, Debug)]
pub enum ToolchainCommands {
    Bootstrap(BootstrapOptions),
    /// Explicitly pull down the toolchain that corresponds with the current submodules
    Pull,
    //NOTE: Not sure if this should be an option or should be explicit or maybe even both
    /// Will look at the bootstrapped toolchain and prune if necessary
    Prune,
    // purely just for testing work
    Test,
}

pub fn handle_cli(subcommand: ToolchainCommands) -> anyhow::Result<()> {
    match subcommand {
        ToolchainCommands::Bootstrap(opts) => do_bootstrap(opts),
        ToolchainCommands::Pull => {
            todo!("implement this later")
        }
        ToolchainCommands::Prune => prune_toolchain(),
        ToolchainCommands::Test => compress_toolchain(),
    }
}

fn get_rust_commit() -> anyhow::Result<String> {
    let repo = git2::Repository::open("toolchain/src/rust")?;
    let cid = repo.head()?.peel_to_commit()?.id();
    Ok(cid.to_string())
}

fn get_abi_version() -> anyhow::Result<semver::Version> {
    let toml = cargo_toml::Manifest::from_path("src/abi/rt-abi/Cargo.toml")?;
    let abipkg = toml.package.as_ref().unwrap();
    Ok(semver::Version::parse(&abipkg.version.get().unwrap())?)
}

static STAMP_PATH: &str = "toolchain/install/stamp";
static NEXT_STAMP_PATH: &str = "toolchain/install/.next_stamp";
fn write_stamp(path: &str, rust_cid: &String, abi_vers: &String) -> anyhow::Result<()> {
    std::fs::write(path, &format!("{}/{}", rust_cid, abi_vers))?;
    Ok(())
}

fn move_stamp() -> anyhow::Result<()> {
    std::fs::rename(NEXT_STAMP_PATH, STAMP_PATH)?;
    Ok(())
}

pub fn needs_reinstall() -> anyhow::Result<bool> {
    let stamp = std::fs::metadata(STAMP_PATH);
    if stamp.is_err() {
        return Ok(true);
    }
    let stamp_data = std::fs::read_to_string(STAMP_PATH)?;
    let vers = stamp_data.split("/").collect::<Vec<_>>();
    if vers.len() != 2 {
        eprintln!("WARNING -- stamp file has invalid format.");
        return Ok(true);
    }

    let rust_commit = get_rust_commit()?;
    let abi_version = get_abi_version()?;
    // TODO: in the future, we'll want to do a full ABI semver req check here. For now
    // we'll just do simple equality checking, especially during development when the
    // ABI may be changing often anyway, and is pre-1.0.
    if vers[0] != rust_commit || vers[1] != abi_version.to_string() {
        eprintln!("WARNING -- Your toolchain is out of date. This is probably because");
        eprintln!("        -- the repository updated to a new rustc commit, or the ABI");
        eprintln!("        -- files were updated.");
        eprintln!("Installed rust toolchain commit: {}", vers[0]);
        eprintln!("toolchain/src/rust: HEAD commit: {}", rust_commit);
        eprintln!("Installed toolchain has ABI version: {}", vers[1]);
        eprintln!("src/abi/rt-abi provides ABI version: {}", abi_version);
        eprintln!("note -- currently the ABI version check requires exact match, not semver.");
        return Ok(true);
    }

    Ok(false)
}

fn build_crtx(name: &str, build_info: &Triple) -> anyhow::Result<()> {
    let objname = format!("{}.o", name);
    let srcname = format!("{}.rs", name);
    let sourcepath = Path::new("toolchain/src/").join(srcname);
    let objpath = format!(
        "toolchain/install/lib/rustlib/{}/lib/self-contained/{}",
        build_info.to_string(),
        objname
    );
    let objpath = Path::new(&objpath);
    println!("building {:?} => {:?}", sourcepath, objpath);
    let status = Command::new("toolchain/install/bin/rustc")
        .arg("--emit")
        .arg("obj")
        .arg("-o")
        .arg(objpath)
        .arg(sourcepath)
        .arg("--crate-type")
        .arg("staticlib")
        .arg("-C")
        .arg("panic=abort")
        .arg("--target")
        .arg(build_info.to_string())
        .status()?;
    if !status.success() {
        anyhow::bail!("failed to compile {}::{}", name, build_info.to_string());
    }

    Ok(())
}

async fn download_efi_files(client: &Client) -> anyhow::Result<()> {
    // efi binaries for x86 machines
    download_file(
        client,
        "http://twizzler.io/dist/bootfiles/OVMF.fd",
        "toolchain/install/OVMF.fd",
    )
    .await?;
    download_file(
        client,
        "http://twizzler.io/dist/bootfiles/BOOTX64.EFI",
        "toolchain/install/BOOTX64.EFI",
    )
    .await?;
    // efi binaries for aarch64 machines
    download_file(
        client,
        "http://twizzler.io/dist/bootfiles/QEMU_EFI.fd",
        "toolchain/install/OVMF-AA64.fd",
    )
    .await?;
    download_file(
        client,
        "http://twizzler.io/dist/bootfiles/BOOTAA64.EFI",
        "toolchain/install/BOOTAA64.EFI",
    )
    .await?;

    Ok(())
}

pub fn set_dynamic(target: &Triple) {
    // This is a bit of a cursed linker line, but it's needed to work around some limitations in
    // rust's linkage support.
    let sysroot_path = Path::new(&format!(
        "toolchain/install/sysroots/{}/lib",
        target.to_string()
    ))
    .canonicalize()
    .unwrap();
    let args = format!("-C prefer-dynamic=y -Z staticlib-prefer-dynamic=y -C link-arg=--allow-shlib-undefined -C link-arg=--undefined-glob=__TWIZZLER_SECURE_GATE_* -C link-arg=--export-dynamic-symbol=__TWIZZLER_SECURE_GATE_* -C link-arg=--warn-unresolved-symbols -Z pre-link-arg=-L -Z pre-link-arg={} -L {}", sysroot_path.display(), sysroot_path.display());
    std::env::set_var("RUSTFLAGS", args);
    std::env::set_var("CARGO_TARGET_DIR", "target/dynamic");
}

pub fn set_static() {
    std::env::set_var(
        "RUSTFLAGS",
        "-C prefer-dynamic=n -Z staticlib-prefer-dynamic=n -C target-feature=+crt-static -C relocation-model=static",
    );
    std::env::set_var("CARGO_TARGET_DIR", "target/static");
}

pub(crate) fn init_for_build(abi_changes_ok: bool) -> anyhow::Result<()> {
    if needs_reinstall()? && !abi_changes_ok {
        eprintln!("!! You'll need to recompile your toolchain. Running `cargo bootstrap` should resolve the issue.");
        anyhow::bail!("toolchain needs reinstall: run cargo bootstrap.");
    }
    std::env::set_var("RUSTC", &get_rustc_path()?);
    std::env::set_var("RUSTDOC", &get_rustdoc_path()?);
    std::env::set_var("CARGO_CACHE_RUSTC_INFO", "0");
    let current_dir = std::env::current_dir().unwrap();
    std::env::set_var("PYTHONPATH", current_dir.join("toolchain/install/python"));

    let compiler_rt_path = "toolchain/src/rust/src/llvm-project/compiler-rt";
    std::env::set_var(
        "RUST_COMPILER_RT_ROOT",
        Path::new(compiler_rt_path).canonicalize().unwrap(),
    );

    let path = std::env::var("PATH").unwrap();
    let lld_bin = get_lld_bin(guess_host_triple().unwrap())?;
    let llvm_bin = get_llvm_bin(guess_host_triple().unwrap())?;
    let rustlib_bin = get_rustlib_bin(guess_host_triple().unwrap())?;
    std::env::set_var(
        "PATH",
        format!(
            "{}:{}:{}:{}:{}",
            rustlib_bin.to_string_lossy(),
            lld_bin.to_string_lossy(),
            llvm_bin.to_string_lossy(),
            std::fs::canonicalize("toolchain/install/bin")
                .unwrap()
                .to_string_lossy(),
            path
        ),
    );
    Ok(())
}

fn generate_config_toml() -> anyhow::Result<()> {
    /* We need to add two(ish) things to the config.toml for rustc: the paths of tools for each twizzler target (built by LLVM as part
    of rustc), and the host triple (added to the list of triples to support). */
    let mut data = File::open("toolchain/src/config.toml")?;
    let mut buf = String::new();
    data.read_to_string(&mut buf)?;
    let commented =
        String::from("# This file was auto-generated by xtask. Do not edit directly.\n") + &buf;
    let mut toml = commented.parse::<DocumentMut>()?;
    let host_triple = guess_host_triple().unwrap();
    let llvm_bin = get_llvm_bin(host_triple)?;
    toml["build"]["target"]
        .as_array_mut()
        .unwrap()
        .push(host_triple);

    let host_cc = std::env::var("CC").unwrap_or("/usr/bin/clang".to_string());
    let host_cxx = std::env::var("CXX").unwrap_or("/usr/bin/clang++".to_string());
    let host_ld = std::env::var("LD").unwrap_or("/usr/bin/clang++".to_string());
    toml["target"][host_triple]["llvm-has-rust-patches"] = toml_edit::value(true);
    toml["target"][host_triple]["cc"] = toml_edit::value(host_cc);
    toml["target"][host_triple]["cxx"] = toml_edit::value(host_cxx);
    toml["target"][host_triple]["linker"] = toml_edit::value(host_ld);

    for triple in all_possible_platforms() {
        let clang = llvm_bin.join("clang").to_str().unwrap().to_string();
        // Use the C compiler as the linker.
        let linker = get_rust_lld(host_triple)?.to_str().unwrap().to_string();
        let clangxx = llvm_bin.join("clang++").to_str().unwrap().to_string();
        let ar = llvm_bin.join("llvm-ar").to_str().unwrap().to_string();

        let tstr = &triple.to_string();
        toml["target"][tstr]["cc"] = toml_edit::value(clang);
        toml["target"][tstr]["cxx"] = toml_edit::value(clangxx);
        toml["target"][tstr]["linker"] = toml_edit::value(linker);
        toml["target"][tstr]["ar"] = toml_edit::value(ar);

        toml["target"][tstr]["llvm-has-rust-patches"] = toml_edit::value(true);
        toml["target"][tstr]["llvm-libunwind"] = toml_edit::value("in-tree");

        toml["build"]["target"].as_array_mut().unwrap().push(tstr);
    }

    let mut out = File::create("toolchain/src/rust/bootstrap.toml")?;
    out.write_all(toml.to_string().as_bytes())?;
    Ok(())
}
