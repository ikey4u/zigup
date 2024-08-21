use std::{
    env::consts,
    fs::{File, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::Path,
};

use anyhow::{anyhow, ensure, Context};
use clap::{Parser, Subcommand};
use serde_json::Value;

pub type Result<T> = anyhow::Result<T, anyhow::Error>;

#[derive(Parser)]
#[clap(author, version, about, verbatim_doc_comment, long_about = None, arg_required_else_help(true))]
struct Cli {
    #[command(subcommand)]
    command: Option<Action>,
    #[arg(long)]
    proxy: Option<String>,
}

#[derive(Subcommand)]
enum Action {
    Update {
        #[arg(short, long)]
        version: Option<String>,
    },
}

fn unzip_tar_xz<S: AsRef<Path>, D: AsRef<Path>>(src: S, dst: D) -> Result<()> {
    use std::{fs::File, io::BufReader};

    use liblzma::read::XzDecoder;
    use tar::Archive;
    let src = src.as_ref();
    let dst = dst.as_ref();

    let file = File::open(src)?;
    let buf_reader = BufReader::new(file);

    let xz_decoder = XzDecoder::new(buf_reader);
    let mut archive = Archive::new(xz_decoder);
    archive.unpack(dst)?;

    Ok(())
}

fn update(proxy: Option<String>, version: Option<String>) -> Result<()> {
    let url = "https://ziglang.org/download/index.json";
    let mut builder = reqwest::blocking::Client::builder();
    if let Some(proxy) = proxy {
        builder = builder.proxy(reqwest::Proxy::all(proxy)?);
    }
    let client = builder.build()?;
    let meta = client
        .get(url)
        .send()
        .context(format!("GET {url}"))?
        .json::<Value>()
        .context(format!("convert response from {url} to json"))?;
    let meta = meta
        .as_object()
        .context("convert zig version metada into dict")?;

    let versions = meta.keys().map(|k| k.as_str()).collect::<Vec<&str>>();
    let version = if let Some(version) = version {
        ensure!(
            versions.iter().any(|&x| x == version.as_str()),
            "provided version {} is not found from existed versions: {:?}",
            version,
            versions
        );
        version
    } else {
        let mut versions = versions
            .iter()
            .filter(|&&x| semver::Version::parse(x).is_ok())
            .collect::<Vec<&&str>>();
        versions.sort_by_key(|x| semver::Version::parse(x).unwrap());
        versions
            .last()
            .context("get latest zig version")?
            .to_string()
    };

    let arch = match consts::ARCH {
        "x86" | "x86_64" | "aarch64" => consts::ARCH,
        _ => {
            return Err(anyhow!(
                "Your system arch {} is not supported yet",
                consts::ARCH
            ))
        }
    };
    let os = match consts::OS {
        "linux" | "macos" | "windows" => consts::OS,
        _ => {
            return Err(anyhow!(
                "Your system type of {} is not supported yet",
                consts::OS
            ))
        }
    };
    let Some(body) = meta.get(&version) else {
        return Err(anyhow!("version is not found in metadata from {url}"));
    };
    let dlurl = body
        .pointer(&format!("/{arch}-{os}/tarball"))
        .context(format!(
            "get zig package url for version {version} ({arch}-{os})"
        ))?
        .as_str()
        .context("use zig package url in string format")?;

    println!("install selected zig version {version} from {dlurl} ...");
    let pkgname = dlurl.split("/").last().context("get zig package name")?;
    let buffer = client
        .get(dlurl)
        .send()
        .context(format!("GET {dlurl}"))?
        .bytes()?;
    let mut f = File::create(pkgname)?;
    f.write_all(&buffer)?;

    // TODO: process case that ~/.cargo does not exist
    let cargo_dir = dirs::home_dir()
        .context("get home directory")?
        .join(".cargo");
    let zig_home = cargo_dir.join("bin").join("app").join("zig");
    // TODO: Windows zig use .zip as package format
    unzip_tar_xz(pkgname, &zig_home).context(format!(
        "decompress {} to {}",
        pkgname,
        zig_home.display()
    ))?;
    let zig_wrapper = cargo_dir.join("bin").join("zig");
    let mut f = OpenOptions::new()
        .mode(0o777)
        .create(true)
        .truncate(true)
        .write(true)
        .open(&zig_wrapper)
        .context(format!(
            "create zig wrapper script: {}",
            zig_wrapper.display()
        ))?;
    let mut content = "#!/usr/bin/env bash\n".to_string();
    content.push_str(&format!(
        "{} $*\n",
        zig_home
            .join(pkgname.trim_end_matches(".tar.xz"))
            .join("zig")
            .display()
    ));
    f.write_all(content.as_bytes())?;
    _ = std::fs::remove_file(pkgname);
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Some(Action::Update { version }) = cli.command.as_ref() {
        update(cli.proxy, version.clone()).context("update zig installtion")?;
    }
    Ok(())
}
