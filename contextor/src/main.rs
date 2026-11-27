mod extract;
mod index;
mod server;

use std::fmt::{self, Display};
use std::io::{self, BufWriter, Write};
use std::net::SocketAddr;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

const DEFAULT_LIMIT: u32 = 30;

#[derive(Parser)]
#[command(version, about = "Find feature annotations in the current directory")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create or update the index for the current directory.
    Index,
    /// Serve read-only queries over HTTP for the current directory.
    Serve {
        #[arg(long, default_value = "127.0.0.1:8080")]
        listen: SocketAddr,
    },
    /// List all distinct feature identifiers in the existing index.
    Features {
        #[arg(long, value_enum, default_value = "text")]
        format: Format,
    },
    /// Find targets with an exact feature identifier in the existing index.
    Query {
        identifier: String,
        #[arg(long, default_value_t = DEFAULT_LIMIT)]
        limit: u32,
        #[arg(long, value_enum, default_value = "text")]
        format: Format,
    },
}

#[derive(Clone, Copy, Default, Deserialize, Serialize, ToSchema, ValueEnum)]
#[serde(rename_all = "lowercase")]
enum Format {
    #[default]
    Text,
    Json,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct Match {
    /// Path relative to the indexed project directory.
    path: String,
    /// Symbol name, qualified for members where applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    symbol_name: Option<String>,
    /// First line of the match, one-based and inclusive.
    #[serde(skip_serializing_if = "Option::is_none")]
    start_line: Option<u32>,
    /// Last line of the match, one-based and inclusive.
    #[serde(skip_serializing_if = "Option::is_none")]
    end_line: Option<u32>,
}

impl Display for Match {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(output, "{}", self.path)?;
        if let (Some(start), Some(end)) = (self.start_line, self.end_line) {
            write!(output, ":{start}-{end}")?;
        }
        if let Some(symbol) = &self.symbol_name {
            write!(output, " {symbol}")?;
        }
        Ok(())
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let root = std::env::current_dir()?.canonicalize()?;
    match cli.command {
        Command::Index => index::update(&root),
        Command::Serve { listen } => server::serve(root, listen),
        Command::Features { format } => print_results(&index::features(&root)?, format),
        Command::Query {
            identifier,
            limit,
            format,
        } => print_results(&index::query(&root, &identifier, limit)?, format),
    }
}

fn print_results<T: Display + Serialize>(results: &[T], format: Format) -> Result<()> {
    let mut output = BufWriter::new(io::stdout().lock());
    match format {
        Format::Json => {
            serde_json::to_writer(&mut output, results)?;
            writeln!(output)?;
        }
        Format::Text => write_text_results(&mut output, results)?,
    }
    output.flush()?;
    Ok(())
}

fn write_text_results(output: &mut impl Write, results: &[impl Display]) -> io::Result<()> {
    for result in results {
        writeln!(output, "{result}")?;
    }
    Ok(())
}

fn main() -> Result<()> {
    match run() {
        Err(error)
            if error
                .downcast_ref::<io::Error>()
                .is_some_and(|e| e.kind() == io::ErrorKind::BrokenPipe) =>
        {
            Ok(())
        }
        result => result,
    }
}
