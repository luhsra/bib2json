use std::collections::BTreeMap;
use std::fs::File;
use std::io::{stdout, BufWriter, Write};
use std::path::PathBuf;

use biblatex::{Bibliography, Chunk, Entry, Person};
use clap::Parser;
use serde::Serialize;

/// Parse bibtex into JSON (using the Typst biblatex crate).
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// input bibtex file
    input: PathBuf,

    #[arg(short, long)]
    /// output file, default: stdout
    output: Option<PathBuf>,
}

#[derive(Serialize, Debug)]
struct SRAPerson {
    first_name: String,
    last_name: String,
}

impl From<Person> for SRAPerson {
    fn from(person: Person) -> Self {
        SRAPerson {
            first_name: person.given_name,
            last_name: [person.prefix, person.name, person.suffix]
                .into_iter()
                .filter(|p| !p.is_empty())
                .collect::<Vec<String>>()
                .join(" "),
        }
    }
}

#[derive(Serialize, Debug)]
struct SRAEntry {
    id: String,
    authors: Vec<SRAPerson>,
    editors: Vec<SRAPerson>,
    entry_type: String,
    bibtex: String,

    #[serde(flatten)]
    other: BTreeMap<String, String>,
}

impl SRAEntry {
    fn entry_to_sra_fields(from: &Entry) -> impl Iterator<Item = (String, String)> + '_ {
        from.fields.iter().map(|(key, value)| {
            let value = value
                .iter()
                .map(|v| match &v.v {
                    Chunk::Math(s) => format!("${s}$"),
                    c => c.get().to_owned(),
                })
                .collect();
            (key.to_owned(), value)
        })
    }

    fn from(e: &Entry, bib: &Bibliography) -> Self {
        SRAEntry {
            id: e.key.to_owned(),
            authors: e.author().map_or(Vec::new(), |authors| {
                authors.into_iter().map(SRAPerson::from).collect()
            }),
            editors: e.editors().map_or(Vec::new(), |editors| {
                editors
                    .into_iter()
                    .flat_map(|tup| tup.0)
                    .map(SRAPerson::from)
                    .collect()
            }),
            entry_type: e.entry_type.to_string(),
            bibtex: e.to_biblatex_string(),
            other: BTreeMap::from_iter(
                e.parents()
                    .unwrap()
                    .iter()
                    .map(|e| bib.get(e).unwrap())
                    .flat_map(Self::entry_to_sra_fields)
                    // Own fields overwrite parent ones
                    .chain(Self::entry_to_sra_fields(e)),
            ),
        }
    }
}

#[derive(Serialize, Debug)]
struct SRABib {
    #[serde(flatten)]
    entries: BTreeMap<String, SRAEntry>,
}

impl SRABib {
    fn new(bib: &Bibliography) -> Self {
        let entries = bib
            .iter()
            .map(|e| (e.key.clone(), SRAEntry::from(e, bib)))
            .collect();

        Self { entries }
    }
}

fn main() -> Result<(), std::io::Error> {
    let args = Args::parse();

    let content = std::fs::read_to_string(args.input)?;
    let bibliography = Bibliography::parse(&content).unwrap();

    let sra_bib = SRABib::new(&bibliography);

    let writer: Box<dyn Write> = if let Some(output) = args.output {
        let file = File::create(output)?;
        Box::new(file)
    } else {
        Box::new(stdout())
    };
    serde_json::to_writer(BufWriter::new(writer), &sra_bib)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use biblatex::Bibliography;

    use crate::SRABib;

    #[test]
    fn crossref() {
        let bib = r#"
            @inproceedings{foo,
                author = {Max Müller},
                title = {Lorem Ipsum et Dolor},
                month = sep,
                year = 2005,
                crossref = {ref},
            }
            @proceedings{ref,
                month = jan,
                year = 2001,
                title = {Book Title},
                category = {baz},
            }
        "#;
        let parsed = Bibliography::parse(bib).unwrap();
        println!("{parsed:#?}");
        let sra_bib = SRABib::new(&parsed);
        println!("{sra_bib:#?}");

        let thesis = &sra_bib.entries["foo"];
        assert_eq!(thesis.entry_type, "inproceedings");
        assert_eq!(thesis.authors.len(), 1);
        assert_eq!(thesis.other["title"], "Lorem Ipsum et Dolor");
        assert_eq!(thesis.other["year"], "2005");
        assert_eq!(thesis.other["month"], "September");
        assert_eq!(thesis.other["category"], "baz");
    }

    #[test]
    fn bib_example() {
        let bib = r#"
            @proceedings{ASE2023,
                title       = {Proceedings of the 38th IEEE/ACM International Conference on Automated Software Engineering},
                year        = 2023,
                publisher   = {IEEE},
                address     = {San Francisco, California, USA},
            }
            @inproceedings{Smith2023,
                author      = {John Smith},
                title       = {Automated Code Generation: Innovations and Challenges},
                pages       = {15-29},
                crossref    = {ASE2023},
            }
            @inproceedings{Doe2023,
                author      = {Jane Doe},
                title       = {Towards a New Era of Software Testing},
                pages       = {30-45},
                crossref    = {ASE2023},
            }
        "#;
        let parsed = Bibliography::parse(bib).unwrap();
        let sra_bib = SRABib::new(&parsed);

        let smith23 = &sra_bib.entries["Smith2023"];
        assert_eq!(smith23.other["booktitle"], "Proceedings of the 38th IEEE/ACM International Conference on Automated Software Engineering");
        assert_eq!(smith23.other["address"], "San Francisco, California, USA");
        assert_eq!(smith23.other["year"], "2023");
        assert_eq!(smith23.other["publisher"], "IEEE");

        let doe23 = &sra_bib.entries["Doe2023"];
        assert_eq!(doe23.other["booktitle"], "Proceedings of the 38th IEEE/ACM International Conference on Automated Software Engineering");
        assert_eq!(doe23.other["address"], "San Francisco, California, USA");
        assert_eq!(doe23.other["year"], "2023");
        assert_eq!(doe23.other["publisher"], "IEEE");
    }
}
