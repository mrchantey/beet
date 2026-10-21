"""Resolve conflict hunks by a word-level three-way merge.

usage: word_merge.py FILE...

A markdown paragraph is one line, so two sides editing different sentences of
it conflict on the whole paragraph. This merges each hunk word by word with
`git merge-file`, rewriting FILE with every hunk that merges clean and
reporting the rest. Hunks need their base: re-materialise them first with
`git checkout --conflict=diff3 -- FILE`. A clean merge can still be wrong (both
sides renaming one thing), so read every result.
"""
import os, re, subprocess, sys, tempfile

HUNK = re.compile(r'<<<<<<< [^\n]*\n(.*?)\|\|\|\|\|\|\| [^\n]*\n(.*?)=======\n(.*?)>>>>>>> [^\n]*\n', re.S)
# a newline stands in its own token line; any NUL would make git see binary
NEWLINE = '␞NL␞'


def tokens(text):
    """One token per line: each word with its trailing space, each newline."""
    return re.findall(r'\n|[^ \n]+ ?| ', text)


def merge(ours, base, theirs):
    """The word-level merge of one hunk, or `None` when it conflicts."""
    with tempfile.TemporaryDirectory() as tmp:
        paths = []
        for label, text in (('ours', ours), ('base', base), ('theirs', theirs)):
            paths.append(os.path.join(tmp, label))
            with open(paths[-1], 'w') as file:
                file.write(''.join(token.replace('\n', NEWLINE) + '\n' for token in tokens(text)))
        out = subprocess.run(['git', 'merge-file', '-p', *paths], capture_output=True, text=True)
    if out.returncode != 0:
        return None
    return ''.join(line.replace(NEWLINE, '\n') for line in out.stdout.split('\n'))


for path in sys.argv[1:]:
    with open(path) as file:
        text = file.read()
    if '<<<<<<< ' in text and not HUNK.search(text):
        sys.exit(f'{path}: hunks carry no base, run `git checkout --conflict=diff3 -- {path}` first')
    unresolved = []

    def resolve(hunk):
        merged = merge(*hunk.groups())
        if merged is None:
            unresolved.append(hunk)
            return hunk.group(0)
        return merged

    text = HUNK.sub(resolve, text)
    with open(path, 'w') as file:
        file.write(text)
    print(f'{path}: ' + (f'{len(unresolved)} hunk(s) left conflicted' if unresolved else 'every hunk merged'))
