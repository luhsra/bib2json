use std::collections::HashMap;
use std::fs::File;
use std::io::{stdout, BufWriter, Write};
use std::path::PathBuf;

use biblatex::{Bibliography, Entry, Person};
use clap::Parser;
use serde::Serialize;

/// Parse bibtex into JSON.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// input bibtex file
    input: PathBuf,

    #[arg(short, long)]
    /// output file, default: stdout
    output: Option<PathBuf>,
}

#[derive(Serialize)]
struct SRAPerson {
    first_name: String,
    last_name: String,
}

impl From<Person> for SRAPerson {
    fn from(person: Person) -> Self {
        SRAPerson {
            first_name: person.given_name,
            last_name: person.name,
        }
    }
}

#[derive(Serialize)]
struct SRAEntry {
    id: String,
    authors: Vec<SRAPerson>,
    entry_type: String,
    bibtex: String,

    #[serde(flatten)]
    other: HashMap<String, String>,
}

impl From<&Entry> for SRAEntry {
    fn from(e: &Entry) -> Self {
        SRAEntry {
            id: e.key.to_owned(),
            authors: e.author().map_or_else(
                |_| Vec::new(),
                |authors| authors.into_iter().map(SRAPerson::from).collect(),
            ),
            entry_type: e.entry_type.to_string(),
            bibtex: e.to_biblatex_string(),
            other: {
                e.fields
                    .iter()
                    .map(|(key, value)| (key.to_owned(), value.iter().map(|v| v.v.get()).collect()))
                    .collect()
            },
        }
    }
}

#[derive(Serialize)]
struct SRABib {
    #[serde(flatten)]
    entries: HashMap<String, SRAEntry>,
}

impl SRABib {
    fn from_bib(bib: &Bibliography) -> Self {
        let entries = bib.iter().map(|e| (e.key.clone(), e.into())).collect();

        Self { entries }
    }
}

fn main() -> Result<(), std::io::Error> {
    let args = Args::parse();

    let content = std::fs::read_to_string(args.input)?;
    let bibliography = Bibliography::parse(&content).unwrap();

    let sra_bib = SRABib::from_bib(&bibliography);

    let writer: BufWriter<Box<dyn Write>> = if let Some(output) = args.output {
        let file = File::create(output)?;
        BufWriter::new(Box::new(file))
    } else {
        BufWriter::new(Box::new(stdout()))
    };
    serde_json::to_writer(writer, &sra_bib)?;

    Ok(())
}
