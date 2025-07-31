use std::{
    fs::{remove_dir_all, File},
    io::Write,
    process::Command,
    vec,
};

use fs_extra::dir::CopyOptions;
use guess_host_triple::guess_host_triple;
use reqwest::Client;

use super::{utils::install_build_tools, BootstrapOptions};
use crate::{
    toolchain::{
        build_crtx, compress_toolchain, download_efi_files, generate_config_toml, get_abi_version,
        get_rust_commit, move_stamp, prune_toolchain, write_stamp, NEXT_STAMP_PATH,
    },
    triple::all_possible_platforms,
};

mod paths;
use paths::*;
mod mover;

pub(crate) fn do_bootstrap(cli: BootstrapOptions) -> anyhow::Result<()> {
    fs_extra::dir::create_all("toolchain/install", false)?;
    if !cli.skip_downloads {
        let client = Client::new();
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(download_efi_files(&client))?;
    }

    install_build_tools(&cli)?;
    let current_dir = std::env::current_dir().unwrap();
    std::env::set_var("PYTHONPATH", current_dir.join("toolchain/install/python"));

    let _ = std::fs::remove_file("toolchain/src/rust/config.toml");
    generate_config_toml()?;

    let _ = fs_extra::dir::remove("toolchain/src/rust/library/twizzler-abis");
    let status = Command::new("cp")
        .arg("-R")
        .arg("src/abi")
        .arg("toolchain/src/rust/library/twizzler-abis")
        .status()?;
    if !status.success() {
        anyhow::bail!("failed to copy twizzler ABI files");
    }

    println!("copying headers");
    let status = Command::new("cp")
        .arg("-R")
        .arg("src/abi/include")
        .arg("toolchain/src/mlibc/sysdeps/twizzler")
        .status()?;
    if !status.success() {
        anyhow::bail!("failed to copy twizzler ABI headers");
    }

    let path = std::env::var("PATH").unwrap();
    let lld_bin = get_lld_bin(guess_host_triple().unwrap())?;
    std::env::set_var(
        "PATH",
        format!(
            "{}:{}:{}:{}",
            lld_bin.to_string_lossy(),
            std::fs::canonicalize("toolchain/install/bin")
                .unwrap()
                .to_string_lossy(),
            std::fs::canonicalize("toolchain/install/python/bin")
                .unwrap()
                .to_string_lossy(),
            path
        ),
    );

    for target_triple in all_possible_platforms() {
        let current_dir = std::env::current_dir().unwrap();
        let sysroot_dir = current_dir.join(format!(
            "toolchain/install/sysroots/{}",
            target_triple.to_string()
        ));
        let build_dir_name = format!("build-{}", target_triple.to_string());
        let src_dir = current_dir.join("toolchain/src/mlibc");
        let build_dir = src_dir.join(&build_dir_name);
        let cross_file = format!("{}/meson-cross-twizzler.txt", sysroot_dir.display());

        std::fs::create_dir_all(&sysroot_dir)?;

        let mut cf = File::create(&cross_file)?;

        writeln!(&mut cf, "[binaries]")?;
        for tool in [
            ("c", "clang"),
            ("cpp", "clang++"),
            ("ar", "llvm-ar"),
            ("strip", "llvm-strip"),
        ] {
            let path = current_dir.join("toolchain/install/bin");
            let path = path.join(tool.1);
            writeln!(&mut cf, "{} = '{}'", tool.0, path.display())?;
        }

        writeln!(&mut cf, "[built-in options]")?;
        let lld_path = current_dir.join("toolchain/src/rust/build/host/lld/bin");
        for tool in ["c_args", "c_link_args", "cpp_args", "cpp_link_args"] {
            writeln!(
                &mut cf,
                "{} = ['-B{}', '-isysroot', '{}', '--sysroot', '{}', '-target', '{}']",
                tool,
                lld_path.display(),
                sysroot_dir.display(),
                sysroot_dir.display(),
                target_triple.to_string()
            )?;
        }

        writeln!(&mut cf, "[host_machine]")?;
        writeln!(&mut cf, "system = 'twizzler'")?;
        writeln!(&mut cf, "cpu_family = '{}'", target_triple.arch.to_string())?;
        writeln!(&mut cf, "cpu = '{}'", target_triple.arch.to_string())?;
        writeln!(&mut cf, "endian = 'little'")?;
        drop(cf);

        let _ = remove_dir_all(&build_dir);
        let status = Command::new("meson")
            .arg("setup")
            .arg(format!("-Dprefix={}", sysroot_dir.display()))
            .arg("-Dheaders_only=true")
            .arg("-Ddefault_library=static")
            .arg(format!("--cross-file={}", &cross_file))
            .arg("--buildtype=debugoptimized")
            .arg(&build_dir)
            .current_dir(current_dir.join("toolchain/src/mlibc"))
            .status()?;
        if !status.success() {
            anyhow::bail!("failed to setup mlibc (headers only)");
        }

        let status = Command::new("meson")
            .arg("install")
            .arg("-q")
            .arg("-C")
            .arg(&build_dir)
            .current_dir(current_dir.join("toolchain/src/mlibc"))
            .status()?;
        if !status.success() {
            anyhow::bail!("failed to install mlibc headers");
        }
    }
    let current_dir = std::env::current_dir().unwrap();
    let builtin_headers =
        current_dir.join("toolchain/src/rust/build/host/llvm/lib/clang/20/include/");
    std::env::set_var("TWIZZLER_ABI_BUILTIN_HEADERS", builtin_headers);

    let keep_args = if cli.keep_early_stages {
        vec![
            "--keep-stage",
            "0",
            "--keep-stage-std",
            "0",
            "--keep-stage",
            "1",
            "--keep-stage-std",
            "1",
        ]
    } else {
        vec![]
    };

    std::env::set_var("BOOTSTRAP_SKIP_TARGET_SANITY", "1");

    if !cli.skip_rust {
        let status = Command::new("./x.py")
            .arg("install")
            .args(&keep_args)
            .current_dir("toolchain/src/rust")
            .status()?;
        if !status.success() {
            anyhow::bail!("failed to compile rust toolchain");
        }

        let src_status = Command::new("./x.py")
            .arg("install")
            .arg("src")
            .args(keep_args)
            .current_dir("toolchain/src/rust")
            .status()?;
        if !src_status.success() {
            anyhow::bail!("failed to install rust source");
        }
    }

    for target in &crate::triple::all_possible_platforms() {
        build_crtx("crti", target)?;
        build_crtx("crtn", target)?;
        let target = target.to_string();
        println!(
            "Copy: {} -> {}",
            get_llvm_native_runtime(&target)?.display(),
            get_llvm_native_runtime_install(&target)?.display()
        );

        let _ =
            std::fs::create_dir_all(get_llvm_native_runtime_install(&target)?.parent().unwrap());

        println!(
            "copying {}",
            get_llvm_native_runtime(&target)?.to_str().unwrap()
        );

        std::fs::copy(
            get_llvm_native_runtime(&target)?,
            get_llvm_native_runtime_install(&target)?,
        )?;

        for name in &["crtbegin", "crtend", "crtbeginS", "crtendS"] {
            let src = format!("toolchain/src/rust/build/{}/native/crt/{}.o", &target, name);
            let dst = format!("toolchain/install/lib/clang/20/lib/{}/{}.o", &target, name);
            std::fs::copy(src, dst)?;
        }
        for name in &["crti", "crtn"] {
            let src = format!(
                "toolchain/install/lib/rustlib/{}/lib/self-contained/{}.o",
                &target, name
            );
            let dst = format!("toolchain/install/lib/clang/20/lib/{}/{}.o", &target, name);
            println!("Copy: {} -> {}", src, dst);
            std::fs::copy(src, dst)?;
        }
        let src = format!("toolchain/install/lib/rustlib/{}/lib/libunwind.a", &target);
        let dst = format!("toolchain/install/lib/clang/20/lib/{}/libunwind.a", &target);
        println!("Copy: {} -> {}", src, dst);
        std::fs::copy(src, dst)?;
    }
    let items = ["bin", "include", "lib", "libexec", "share"]
        .into_iter()
        .map(|name| format!("toolchain/src/rust/build/host/llvm/{}", name))
        .collect::<Vec<_>>();

    println!("copying LLVM toolchain...");
    fs_extra::copy_items(
        &items,
        "toolchain/install",
        &CopyOptions::new().overwrite(true),
    )?;

    let usr_link = "toolchain/install/usr";
    let local_link = "toolchain/install/local";
    let _ = std::fs::remove_file(usr_link);
    std::os::unix::fs::symlink(".", usr_link)?;
    let _ = std::fs::remove_file(local_link);
    std::os::unix::fs::symlink(".", local_link)?;

    for target_triple in all_possible_platforms() {
        let current_dir = std::env::current_dir().unwrap();
        let sysroot_dir = current_dir.join(format!(
            "toolchain/install/sysroots/{}",
            target_triple.to_string()
        ));
        let build_dir_name = format!("build-{}", target_triple.to_string());
        let src_dir = current_dir.join("toolchain/src/mlibc");
        let build_dir = src_dir.join(&build_dir_name);
        let cross_file = format!("{}/meson-cross-twizzler.txt", sysroot_dir.display());

        let _ = remove_dir_all(&build_dir);

        let status = Command::new("meson")
            .arg("setup")
            .arg(format!("-Dprefix={}", sysroot_dir.display()))
            .arg("-Ddefault_library=static")
            .arg(format!("--cross-file={}", cross_file))
            .arg("--buildtype=debugoptimized")
            .arg(&build_dir_name)
            .current_dir(current_dir.join("toolchain/src/mlibc"))
            .status()?;
        if !status.success() {
            anyhow::bail!("failed to setup mlibc");
        }
        let status = Command::new("meson")
            .arg("compile")
            .arg("-C")
            .arg(&build_dir_name)
            .current_dir(current_dir.join("toolchain/src/mlibc"))
            .status()?;
        if !status.success() {
            anyhow::bail!("failed to build mlibc");
        }

        let status = Command::new("meson")
            .arg("install")
            .arg("-q")
            .arg("-C")
            .arg(&build_dir_name)
            .current_dir(current_dir.join("toolchain/src/mlibc"))
            .status()?;
        if !status.success() {
            anyhow::bail!("failed to install mlibc");
        }

        let usr_link = sysroot_dir.join("usr");
        let _ = std::fs::remove_file(&usr_link);
        std::os::unix::fs::symlink(".", usr_link)?;
    }

    let rust_commit = get_rust_commit()?;
    let abi_version = get_abi_version()?;
    write_stamp(NEXT_STAMP_PATH, &rust_commit, &abi_version.to_string())?;
    move_stamp()?;

    if !cli.keep_old_artifacts {
        let res = std::fs::remove_dir_all("target");
        if let Err(e) = res {
            if e.kind() != std::io::ErrorKind::NotFound {
                anyhow::bail!("failed to remove old build artifacts");
            }
        }
    }

    let host_triple = guess_host_triple().unwrap();

    for target_triple in all_possible_platforms() {
        mover::move_all(host_triple, &target_triple.to_string())?;
    }

    if !cli.skip_prune {
        prune_toolchain()?;
    }

    if cli.compress {
        compress_toolchain()?;
    }

    println!("ready!");
    Ok(())
}
