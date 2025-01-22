//! This module contains all CLI-specific code for the host binary.

use crate::{interop::InteropHostCli, single::SingleChainHostCli};
use clap::{
    builder::styling::{AnsiColor, Color, Style},
    ArgAction, Parser, Subcommand,
};
use serde::Serialize;

mod parser;
pub(crate) use parser::{parse_b256, parse_bytes};

mod tracing_util;
pub use tracing_util::init_tracing_subscriber;

const ABOUT: &str = "
kona-host is a CLI application that runs the Kona pre-image server and client program. The host
can run in two modes: server mode and native mode. In server mode, the host runs the pre-image
server and waits for the client program in the parent process to request pre-images. In native
mode, the host runs the client program in a separate thread with the pre-image server in the
primary thread.
";

/// The host binary CLI application arguments.
#[derive(Parser, Serialize, Clone, Debug)]
#[command(about = ABOUT, version, styles = cli_styles())]
pub struct HostCli {
    /// Verbosity level (0-2)
    #[arg(long, short, action = ArgAction::Count)]
    pub v: u8,
    /// Host mode
    #[clap(subcommand)]
    pub mode: HostMode,
}

/// Operation modes for the host binary.
#[derive(Subcommand, Serialize, Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum HostMode {
    /// Run the host in single-chain mode.
    Single(SingleChainHostCli),
    /// Run the host in super-chain (interop) mode.
    Super(InteropHostCli),
}

/// Styles for the CLI application.
pub(crate) const fn cli_styles() -> clap::builder::Styles {
    clap::builder::Styles::styled()
        .usage(Style::new().bold().underline().fg_color(Some(Color::Ansi(AnsiColor::Yellow))))
        .header(Style::new().bold().underline().fg_color(Some(Color::Ansi(AnsiColor::Yellow))))
        .literal(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green))))
        .invalid(Style::new().bold().fg_color(Some(Color::Ansi(AnsiColor::Red))))
        .error(Style::new().bold().fg_color(Some(Color::Ansi(AnsiColor::Red))))
        .valid(Style::new().bold().underline().fg_color(Some(Color::Ansi(AnsiColor::Green))))
        .placeholder(Style::new().fg_color(Some(Color::Ansi(AnsiColor::White))))
}
