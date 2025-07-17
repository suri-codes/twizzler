use std::{
    fs::{remove_dir_all, File},
    io::{Read, Write},
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    vec,
};

use anyhow::Context;
use fs_extra::dir::CopyOptions;
use guess_host_triple::guess_host_triple;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use toml_edit::DocumentMut;

use super::get_toolchain_path;
use crate::{
    triple::{all_possible_platforms, Triple},
    BootstrapOptions,
};

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

pub fn prune_toolchain() -> anyhow::Result<()> {
    let prune_path = format!("{}/prune.txt", get_toolchain_path()?);

    let mut prune_f = File::open(&prune_path)
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
