use std::{
    env::consts,
    fs::{create_dir_all, File, OpenOptions},
    io::Write,
    path::PathBuf,
};

use anyhow::{anyhow, ensure, Context};
use serde_json::Value;

use crate::Result;

const ZIG_META_URL: &str = "https://ziglang.org/download/index.json";

pub struct ZigBuilder {
    version: Option<String>,
    proxy: Option<String>,
}

impl ZigBuilder {
    pub fn new() -> Self {
        Self {
            version: None,
            proxy: None,
        }
    }

    pub fn with_version<S: AsRef<str>>(mut self, version: S) -> Self {
        self.version = Some(version.as_ref().to_string());
        self
    }

    pub fn with_proxy<S: AsRef<str>>(mut self, proxy: S) -> Self {
        self.proxy = Some(proxy.as_ref().to_string());
        self
    }

    pub fn build(self) -> Result<Zig> {
        let home = dirs::home_dir()
            .context("get home directory")?
            .join(".zigup");
        create_dir_all(&home).context(format!(
            "create directory {} for zigup",
            home.display()
        ))?;

        let meta = crate::net::request(ZIG_META_URL, self.proxy.as_deref())?
            .json::<Value>()
            .context(format!("convert response from {ZIG_META_URL} to json"))?;
        let meta = meta
            .as_object()
            .context("convert zig version metada into dict")?;

        let versions = meta.keys().map(|k| k.as_str()).collect::<Vec<&str>>();
        let version = if let Some(version) = self.version {
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

        let version_deails = meta.get(&version).context(format!(
            "version {version} is not found in metadata from {ZIG_META_URL}"
        ))?;
        let dlurl = version_deails
            .pointer(&format!("/{arch}-{os}/tarball"))
            .context(format!(
                "get zig download url for version {version} ({arch}-{os})"
            ))?
            .as_str()
            .context("get zig download url as string")?;

        Ok(Zig {
            version,
            proxy: self.proxy,
            arch: arch.to_string(),
            os: os.to_string(),
            home,
            dlurl: dlurl.to_string(),
        })
    }
}

pub struct Zig {
    version: String,
    arch: String,
    os: String,
    dlurl: String,
    home: PathBuf,
    proxy: Option<String>,
}

impl Zig {
    pub fn install(&self) -> Result<()> {
        println!(
            "install selected zig version {} from {} ...",
            self.version, self.dlurl,
        );
        let pkgname = self
            .dlurl
            .split("/")
            .last()
            .context("get zig package name")?;
        let buffer =
            crate::net::request(self.dlurl.as_str(), self.proxy.clone())
                .context(format!("request data from {}", self.dlurl))?
                .bytes()?;
        let mut f = File::create(pkgname)?;
        f.write_all(&buffer)
            .context(format!("write zig package data to {pkgname}"))?;

        #[cfg(not(windows))]
        {
            use std::os::unix::fs::OpenOptionsExt;

            let current_zig_dir = self
                .home
                .join("current")
                .join(pkgname.trim_end_matches(".tar.xz"));
            crate::packer::unpack_tar_xz(pkgname, self.home.join("current"))
                .context(format!(
                    "decompress {} to {}",
                    pkgname,
                    self.home.display()
                ))?;
            if let Some(cargo_bin_dir) =
                dirs::home_dir().map(|x| x.join(".cargo").join("bin"))
            {
                let zig_wrapper = cargo_bin_dir.join("zig");
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
                    current_zig_dir.join("zig").display()
                ));
                f.write_all(content.as_bytes())?;
                _ = std::fs::remove_file(pkgname);
            } else {
                println!("[+] installation done, add {} to your PATH and restart your shell", current_zig_dir.join("zig").display());
            }
        }

        #[cfg(windows)]
        {
            panic!("does not support yet");
        }

        Ok(())
    }
}
