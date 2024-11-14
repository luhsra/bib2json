from pathlib import Path
import bibtexparser
import sys
import time
import bib2

file = sys.argv[1]

t_start = time.time()
parser = bibtexparser.bparser.BibTexParser(common_strings=True)
parser.ignore_nonstandard_types = False
old_bib = bibtexparser.load(Path(sys.argv[1]).open(), parser).entries_dict
print(f"bibtexparser {len(old_bib)} entries: {time.time() - t_start}")

t_start = time.time()
new_bib = bib2.loads(Path(sys.argv[1]).read_text())
print(f"bib2json {len(new_bib)} entries: {time.time() - t_start}")

assert old_bib.keys() == new_bib.keys()
