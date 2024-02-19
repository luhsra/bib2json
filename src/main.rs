use std::collections::HashMap;
use std::fs::File;
use std::io::stdout;
use std::io::BufWriter;
use std::path::PathBuf;

use biblatex::Bibliography;
use biblatex::Person;
use clap::Parser;
use serde::Serialize;
use serde_json::Value;

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
struct Entry {
    authors: Vec<SRAPerson>,

    #[serde(flatten)]
    other: HashMap<String, Value>,
}

#[derive(Serialize)]
struct SRABib {
    entries: HashMap<String, Entry>,
}

impl SRABib {
    fn from_bib(bib: &Bibliography) -> Self {
        let entries = bib
            .iter()
            .map(|e| {
                (
                    e.key.clone(),
                    Entry {
                        authors: e.author().map_or_else(
                            |_| Vec::new(),
                            |authors| authors.into_iter().map(SRAPerson::from).collect(),
                        ),
                        other: {
                            let mut map = HashMap::new();
                            for (key, value) in e.fields.iter() {
                                if key == "authors" {
                                    continue;
                                }

                                let mut v = String::new();
                                for bar in value {
                                    v.push_str(bar.v.get())
                                }

                                map.insert(key.to_owned(), Value::String(v));
                            }
                            map
                        },
                    },
                )
            })
            .collect();

        Self { entries }
    }
}

fn main() -> Result<(), std::io::Error> {
    let args = Args::parse();

    let content = std::fs::read_to_string(args.input)?;
    let bibliography = Bibliography::parse(&content).unwrap();

    let sra_bib = SRABib::from_bib(&bibliography);

    if let Some(output) = args.output {
        let file = File::create(output)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &sra_bib)?;
    } else {
        let writer = BufWriter::new(stdout());
        serde_json::to_writer(writer, &sra_bib)?;
    }

    Ok(())
}
