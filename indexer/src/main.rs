mod extract;
mod index;

use std::io::{self, BufWriter, Write};

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;

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
    /// Find targets with an exact feature identifier in the existing index.
    Query {
        identifier: String,
        #[arg(long, default_value_t = 30)]
        limit: u32,
        #[arg(long, value_enum, default_value = "text")]
        format: Format,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Json,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Match {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    symbol_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_line: Option<u32>,
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let root = std::env::current_dir()?.canonicalize()?;
    match cli.command {
        Command::Index => index::update(&root),
        Command::Query {
            identifier,
            limit,
            format,
        } => print_matches(index::query(&root, &identifier, limit)?, format),
    }
}

fn print_matches(matches: Vec<Match>, format: Format) -> Result<()> {
    let mut output = BufWriter::new(io::stdout().lock());
    match format {
        Format::Json => {
            serde_json::to_writer(&mut output, &matches)?;
            writeln!(output)?;
        }
        Format::Text => {
            for result in matches {
                write!(output, "{}", result.path)?;
                if let (Some(start), Some(end)) = (result.start_line, result.end_line) {
                    write!(output, ":{start}-{end}")?;
                }
                if let Some(symbol) = result.symbol_name {
                    write!(output, " {symbol}")?;
                }
                writeln!(output)?;
            }
        }
    }
    output.flush()?;
    Ok(())
}

fn main() -> Result<()> {
    match run() {
        Err(error) if error.downcast_ref::<io::Error>().is_some_and(|e| {
            e.kind() == io::ErrorKind::BrokenPipe
        }) => Ok(()),
        result => result,
    }
}
