use std::collections::BTreeMap;
use std::convert::Infallible;
use std::fmt;

use biblatex::{Bibliography, Chunk, Entry, ParseError, Person};
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// The bib2 module.
#[pymodule]
fn bib2(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(loads, m)?)?;
    Ok(())
}

/// Load a BibTeX file from a file path.
#[pyfunction]
fn loads(content: &str) -> PyResult<SRABib> {
    let sra_bib = SRABib::loads(content)?;
    Ok(sra_bib)
}

#[derive(Debug)]
struct Error(ParseError);
impl std::error::Error for Error {}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
impl From<Error> for PyErr {
    fn from(e: Error) -> PyErr {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string())
    }
}
impl From<ParseError> for Error {
    fn from(e: ParseError) -> Self {
        Error(e)
    }
}

#[derive(Debug)]
struct SRAPerson {
    first_name: String,
    last_name: String,
}
impl<'py> IntoPyObject<'py> for SRAPerson {
    type Target = PyDict;
    type Output = Bound<'py, PyDict>;
    type Error = Infallible;
    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        let dict = PyDict::new(py);
        dict.set_item("first_name", &self.first_name).unwrap();
        dict.set_item("last_name", &self.last_name).unwrap();
        Ok(dict)
    }
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

#[derive(Debug)]
struct SRAEntry {
    id: String,
    authors: Vec<SRAPerson>,
    editors: Vec<SRAPerson>,
    entry_type: String,
    bibtex: String,

    other: BTreeMap<String, String>,
}
impl<'py> IntoPyObject<'py> for SRAEntry {
    type Target = PyDict;
    type Output = Bound<'py, PyDict>;
    type Error = Infallible;
    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        let dict = PyDict::new(py);
        for (key, value) in self.other {
            dict.set_item(key, value).unwrap();
        }
        dict.set_item("id", self.id).unwrap();
        dict.set_item("authors", self.authors).unwrap();
        dict.set_item("editors", self.editors).unwrap();
        dict.set_item("entry_type", self.entry_type).unwrap();
        dict.set_item("bibtex", self.bibtex).unwrap();
        Ok(dict)
    }
}
impl SRAEntry {
    fn fields(from: &Entry) -> impl Iterator<Item = (String, String)> + '_ {
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
        // also include crossrefs in bibtex export
        let mut bibtex = e.to_biblatex_string();
        if let Ok(parents) = e.parents() {
            for key in parents {
                if let Some(parent) = bib.get(&key) {
                    bibtex += "\n\n";
                    bibtex += &parent.to_biblatex_string();
                }
            }
        }
        SRAEntry {
            id: e.key.to_owned(),
            authors: e
                .author()
                .unwrap_or_default()
                .into_iter()
                .map(SRAPerson::from)
                .collect(),
            editors: e
                .editors()
                .unwrap_or_default()
                .into_iter()
                .flat_map(|tup| tup.0)
                .map(SRAPerson::from)
                .collect(),
            entry_type: e.entry_type.to_string(),
            bibtex,
            other: e
                .parents() // Add xref and crossref fields
                .unwrap()
                .iter()
                .map(|id| bib.get(id).unwrap())
                .flat_map(Self::fields)
                // Own fields overwrite parent ones
                .chain(Self::fields(e))
                .collect(),
        }
    }
}

#[derive(Debug)]
struct SRABib {
    entries: BTreeMap<String, SRAEntry>,
}
impl<'py> IntoPyObject<'py> for SRABib {
    type Target = PyDict;
    type Output = Bound<'py, PyDict>;
    type Error = Infallible;
    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        let dict = PyDict::new(py);
        for (key, value) in self.entries {
            dict.set_item(key, value).unwrap();
        }
        Ok(dict)
    }
}
impl SRABib {
    fn new(bib: &Bibliography) -> Self {
        let entries = bib
            .iter()
            .map(|e| (e.key.clone(), SRAEntry::from(e, bib)))
            .collect();

        Self { entries }
    }
    fn loads(content: &str) -> Result<Self, Error> {
        let bibliography = Bibliography::parse(content)?;
        Ok(Self::new(&bibliography))
    }
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
        assert_eq!(thesis.other["year"], "2001");
        assert_eq!(thesis.other["month"], "January");
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
