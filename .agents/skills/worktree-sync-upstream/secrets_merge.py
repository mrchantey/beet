"""Three-way merge a conflicted `secrets.toml`, its plaintext held in memory only.

usage: secrets_merge.py [--write] [--take=ours|theirs] [BASE OURS THEIRS]

Run from the repo root mid rebase or merge, with `secrets.toml` conflicted. Reads
the index stages (1 base, 2 ours, 3 theirs: in a rebase, ours is the upstream
and theirs the commit being replayed), or the three revisions given, decrypts every group with the age
identity and merges it record by record: a record changed on one side takes
that side, one changed on both is line merged and checked field by field, and
a field changed differently on both sides aborts, unless --take names the side
whose whole record to keep (the credential the provider holds, ie the one
sealed by the later converge). Prints record and field names, never a value.

With --write it writes the resolved file: each group's index regenerated from
its merged records (the index is the sealed records minus their values), each
group resealed only when its records differ from both sides, then every blob
and index verified against each other.
"""
import os, re, subprocess, sys, tempfile, tomllib

KEYS = os.environ.get('BEET_AGE_IDENTITY', os.path.expanduser('~/.config/beet/age/keys.txt'))
BLOB = re.compile(r'^(\S+) = """\n(-----BEGIN AGE ENCRYPTED FILE-----\n.*?-----END AGE ENCRYPTED FILE-----\n)"""', re.S | re.M)
SECTION = re.compile(r'^\[(groups\.[A-Za-z0-9_-]+|sealed)\]\n', re.M)
RECORD = re.compile(r'^\[secrets\.([A-Za-z0-9_]+)\]\n', re.M)
VALUE = re.compile(r'^value = (""".*?"""|\'\'\'.*?\'\'\'|"(?:[^"\\\n]|\\.)*"|\'[^\'\n]*\')\n', re.S | re.M)


TAKE = next((arg.split('=', 1)[1] for arg in sys.argv[1:] if arg.startswith('--take=')), None)
assert TAKE in (None, 'ours', 'theirs'), '--take is ours or theirs'
REVS = [arg for arg in sys.argv[1:] if not arg.startswith('--')]
assert len(REVS) in (0, 3), 'give all three revisions or none'


def stage(n):
    spec = f'{REVS[n - 1]}:secrets.toml' if REVS else f':{n}:secrets.toml'
    return subprocess.run(['git', 'show', spec], capture_output=True, text=True, check=True).stdout


def decrypt(armored):
    return subprocess.run(['age', '-d', '-i', KEYS], input=armored.encode(), capture_output=True, check=True).stdout.decode()


def encrypt(plain):
    args = ['age', '-a'] + [arg for recipient in tomllib.loads(plain)['recipients'] for arg in ('-r', recipient)]
    return subprocess.run(args, input=plain.encode(), capture_output=True, check=True).stdout.decode()


def records(plain):
    """The text before the first record, and each record with its subtables keyed by name."""
    heads = list(RECORD.finditer(plain))
    bodies = {head.group(1): plain[head.start():(heads[i + 1].start() if i + 1 < len(heads) else len(plain))] for i, head in enumerate(heads)}
    return plain[:heads[0].start()] if heads else plain, bodies


def sections(text):
    """Each group's index region of a document, keyed by group."""
    heads = list(SECTION.finditer(text))
    assert heads and heads[0].start() == 0, 'unexpected text before the first section'
    return {head.group(1)[len('groups.'):]: text[head.start():(heads[i + 1].start() if i + 1 < len(heads) else len(text))] for i, head in enumerate(heads) if head.group(1) != 'sealed'}


def index_of(group, plain, region):
    """The index of `group` as the document keeps it, ending as `region` does."""
    text = '[groups.' + group + ']\n' + re.sub(r'^\[secrets\.', f'[groups.{group}.secrets.', VALUE.sub('', plain), flags=re.M)
    return text.rstrip('\n') + region[len(region.rstrip('\n')):]


def flat(obj, prefix=''):
    if isinstance(obj, dict):
        for key, value in obj.items():
            yield from flat(value, f'{prefix}.{key}' if prefix else key)
    else:
        yield prefix, obj


def fields_of(record):
    """The fields of one record's text, without its `secrets.<name>` header."""
    (fields,) = tomllib.loads(record)['secrets'].values()
    return fields


def pick(base, ours, theirs):
    """The side that changed and its value: `''` for theirs or neither, `'ours'`, or `'both'` (value `None`) when they differ."""
    if ours == theirs or ours == base:
        return theirs, ''
    if theirs == base:
        return ours, 'ours'
    return None, 'both'


def line_merge(name, base, ours, theirs):
    """A record both sides changed, line merged when every field changed on one side alone, else the reason it cannot be."""
    if None in (base, ours, theirs):
        return None, 'added or removed on one side and changed on the other'
    fields = [dict(flat(fields_of(text))) for text in (base, ours, theirs)]
    expect = {}
    for field in set().union(*fields):
        value, side = pick(*(fields_of_side.get(field) for fields_of_side in fields))
        if side == 'both':
            return None, f'`{field}` changed differently on both sides'
        if value is not None:
            expect[field] = value
    with tempfile.TemporaryDirectory() as tmp:
        paths = []
        for label, text in (('ours', ours), ('base', base), ('theirs', theirs)):
            paths.append(os.path.join(tmp, label))
            with open(paths[-1], 'w') as file:
                file.write(text)
        out = subprocess.run(['git', 'merge-file', '-p', *paths], capture_output=True, text=True)
    if out.returncode != 0 or dict(flat(fields_of(out.stdout))) != expect:
        return None, 'a line merge does not reproduce each side\'s fields'
    print(f'{name}: both, fields {sorted(k for k in expect if fields[1].get(k) != fields[0].get(k))} from ours')
    return out.stdout, ''


def merge_record(name, base, ours, theirs):
    merged, side = pick(base, ours, theirs)
    if side != 'both':
        return merged, side
    merged, conflict = line_merge(name, base, ours, theirs)
    if not conflict:
        return merged, side
    if TAKE is None:
        raise SystemExit(f'{name}: {conflict}; merge by hand, or rerun with --take=ours|theirs to keep that side\'s whole record')
    # the named side's whole record, naming what the other side loses
    kept, dropped = (ours, theirs) if TAKE == 'ours' else (theirs, ours)
    lost = sorted({k for k, v in dict(flat(fields_of(dropped))).items() if dict(flat(fields_of(base))).get(k) != v} if dropped and base else set())
    print(f'{name}: {conflict}, took {TAKE}, dropping the other side\'s change to {lost or "the whole record"}')
    return kept, TAKE


texts = {n: stage(n) for n in (1, 2, 3)}
blobs = {n: dict(BLOB.findall(texts[n])) for n in (1, 2, 3)}
assert set(blobs[1]) == set(blobs[2]) == set(blobs[3]), 'a group was added or removed, merge by hand'
merged = {}
for group in blobs[2]:
    plains = {n: decrypt(blobs[n][group]) for n in (1, 2, 3)}
    (pre1, recs1), (pre2, recs2), (pre3, recs3) = (records(plains[n]) for n in (1, 2, 3))
    preamble, side = pick(pre1, pre2, pre3)
    if side == 'both':
        raise SystemExit(f'{group}: recipients changed on both sides, merge by hand')
    out = {}
    for name in recs3 | recs2 | recs1:
        record, side = merge_record(f'{group}.{name}', recs1.get(name), recs2.get(name), recs3.get(name))
        if side == 'ours':
            print(f'{group}.{name}: ours')
        elif side == '' and recs3.get(name) != recs1.get(name):
            print(f'{group}.{name}: theirs')
        if record is not None:
            out[name] = record
    # theirs' record order, then any record ours added
    plain = preamble + ''.join(out[name] for name in list(recs3) + [n for n in recs2 if n not in recs3] if name in out)
    merged[group] = (plain, plains)
    tomllib.loads(plain)

if '--write' in sys.argv:
    text = texts[2]
    regions = sections(text)
    for group, (plain, plains) in merged.items():
        # keep a side's ciphertext when its records are the merge, sparing a reseal
        cipher = next((blobs[n][group] for n in (2, 3) if plains[n] == plain), None) or encrypt(plain)
        text = text.replace(regions[group], index_of(group, plain, regions[group]), 1)
        text, count = re.subn(r'^' + re.escape(group) + r' = """\n-----BEGIN AGE ENCRYPTED FILE-----\n.*?-----END AGE ENCRYPTED FILE-----\n"""', lambda _: f'{group} = """\n{cipher}"""', text, flags=re.S | re.M)
        assert count == 1, group
    # verify: each blob opens to its merge, each index is its records minus values
    tomllib.loads(text)
    regions = sections(text)
    for group, armored in BLOB.findall(text):
        plain = decrypt(armored)
        assert plain == merged[group][0], group
        assert regions[group] == index_of(group, plain, regions[group]), group
    with open('secrets.toml', 'w') as file:
        file.write(text)
    print('written: every blob opens to its merge and every index matches its records')
