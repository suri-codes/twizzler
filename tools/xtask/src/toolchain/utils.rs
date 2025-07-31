use std::{
    fs::{self, File},
    io::{Read, Write},
    process::Command,
};

use anyhow::Context;
use git2::Repository;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;

use super::{get_bin_path, BootstrapOptions};

pub async fn download_file(client: &Client, url: &str, path: &str) -> anyhow::Result<()> {
    use futures_util::StreamExt;
    let res = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("failed to download {}", url))?;
    let total_size = res
        .content_length()
        .with_context(|| format!("failed to get content-length for {}", url))?;
    println!("downloading {}", url);
    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar().template("{prefix}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?.progress_chars("#>-"));

    let mut file = File::create(path).with_context(|| format!("failed to create file {}", path))?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.with_context(|| format!("error while downloading file {}", url))?;
        file.write_all(&chunk)
            .with_context(|| format!("error while writing to file {}", path))?;
        let new = std::cmp::min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }
    pb.finish_and_clear();
    println!("downloaded {} => {}", url, path);
    Ok(())
}

pub fn install_build_tools(_cli: &BootstrapOptions) -> anyhow::Result<()> {
    println!("installing bindgen");
    let status = Command::new("cargo")
        .arg("install")
        .arg("--root")
        .arg("toolchain/install")
        .arg("bindgen-cli")
        .status()?;
    if !status.success() {
        anyhow::bail!("failed to install bindgen");
    }

    println!("installing meson & ninja");
    let status = Command::new("pip3")
        .arg("install")
        .arg("--target")
        .arg("toolchain/install/python")
        .arg("--force-reinstall")
        .arg("--ignore-installed")
        .arg("--no-warn-script-location")
        .arg("--upgrade")
        .arg("meson")
        .arg("ninja")
        .status()?;
    if !status.success() {
        anyhow::bail!("failed to install meson and ninja");
    }

    Ok(())
}

pub fn prune_bins() -> anyhow::Result<()> {
    let wanted_bins = [
        "bindgen",
        "clang",
        "clang++",
        "rust-gdb",
        "rust-gdbgui",
        "rust-lldb",
        "rustc",
        "rustdoc",
        "set-xcode-analyzer",
    ];

    // let mut file_names = Vec::new();
    let bin_path = get_bin_path()?;
    for entry in fs::read_dir(&bin_path)? {
        let entry = entry?;
        if let Some(name) = entry.file_name().to_str() {
            if !wanted_bins.contains(&name) {
                // we delete
                Command::new("rm")
                    .arg(format!("{}/{}", &bin_path, name))
                    .status()?;
            }
        }
    }

    Ok(())
}

pub fn prune_toolchain() -> anyhow::Result<()> {
    // let prune_path = format!("{}/prune.txt", get_toolchain_path()?);
    //TODO: figure out how this is going to work with multiple toolchains
    let prune_path = "toolchain/prune.txt";

    let mut prune_f = File::open(prune_path)
        .with_context(|| format!("was not able to find prune file at path {}", &prune_path))?;

    let mut to_prune = String::new();

    prune_f.read_to_string(&mut to_prune)?;

    for path in to_prune.lines() {
        // A safety check to make sure that we only remove stuff inside toolchain as some
        // destructive operations are ahead
        assert!(path.to_owned().starts_with("./toolchain"));

        let _ = Command::new("rm").args(["-rf", path]).spawn()?;
    }

    Ok(())
}

// example tag for toolchain
// toolchain_<x86|aarch64>_<linux|darwin>_<hash>.tar.zst
pub fn generate_tag() -> anyhow::Result<String> {
    let hash = generate_hash()?;

    let arch = {
        let output = Command::new("uname").arg("-m").output()?;
        let stdout = String::from_utf8(output.stdout)?;
        stdout.trim().to_owned()
    };

    let os = {
        let output = Command::new("uname").output()?;
        let stdout = String::from_utf8(output.stdout)?;
        stdout.trim().to_owned()
    };

    let tag = format!("toolchain_{arch}_{os}_{hash}");

    println!("generated tag:{tag}");

    Ok(tag)
}

fn generate_hash() -> anyhow::Result<String> {
    let repo = Repository::open("./")?;

    let submodules = repo.submodules()?;

    let get_head = |submodule_path: &str| -> String {
        let oid = submodules
            .iter()
            .find(|e| e.name().expect("submodulue should have a name") == submodule_path)
            .unwrap_or_else(|| panic!("submodule not found at path: {}", submodule_path))
            .head_id()
            .expect("head should exist")
            .to_string();

        // truncate the oid to 7 characters, if its good enough for github, its good enough for us
        let (head, _) = oid.split_at(7);
        head.to_owned()
    };

    let rust_head = get_head("toolchain/src/rust");
    let mlibc_head = get_head("toolchain/src/mlibc");
    let abi_head = get_head("src/abi");

    Ok(format!("{rust_head}-{mlibc_head}-{abi_head}"))
}

pub fn compress_toolchain() -> anyhow::Result<()> {
    let tag = generate_tag()?;

    // when we build the toolchain we ideally move everything into install and then compress
    // that no?
    let _ = Command::new("tar")
        .arg("--zstd")
        .arg("-c")
        .arg("-f")
        .arg([tag.as_str(), ".tar.zst"].concat())
        .arg("toolchain/install")
        .spawn()?;

    Ok(())
}
