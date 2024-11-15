from pathlib import Path
import bibtexparser
import sys
import time
import json
from subprocess import check_output

file = sys.argv[1]

t_start = time.time()
parser = bibtexparser.bparser.BibTexParser(common_strings=True)
parser.ignore_nonstandard_types = False
old_bib = bibtexparser.load(Path(sys.argv[1]).open(), parser).entries_dict
print(f"bibtexparser {len(old_bib)} entries: {time.time() - t_start}")

t_start = time.time()
out = check_output([Path(__file__).parent.parent / "target/release/bib2json", sys.argv[1]])
new_bib = json.loads(out)
print(f"bibtexparser {len(new_bib)} entries: {time.time() - t_start}")

assert old_bib.keys() == new_bib.keys()
