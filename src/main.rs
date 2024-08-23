mod net;
mod packer;
mod zig;

use anyhow::Context;
use clap::{Parser, Subcommand};
use zig::ZigBuilder;

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
    Install { version: Option<String> },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Some(Action::Install { version }) = cli.command.as_ref() {
        let mut builder = ZigBuilder::new();
        if let Some(version) = version {
            builder = builder.with_version(version);
        }
        if let Some(proxy) = cli.proxy {
            builder = builder.with_proxy(proxy);
        }
        let zig = builder.build()?;
        zig.install().context("try to install zig ...")?;
    }
    Ok(())
}
