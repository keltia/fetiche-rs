#! /usr/bin/env python3
#
"""
This utility takes a filename or a directory.  If the former, import the given file and if the latter
all parquets files in the tree.
"""

import argparse
import os
import re
from pathlib import Path
from typing import Any

sites = {
    'Bretigny': 'BRE',
    'Luxembourg': 'LUX',
    'Brussels': 'BRU',
    'Belfast': 'BEL',
    'Bordeaux': 'BDX',
    'Gatwick': 'LON',
    'London': 'LON',
    'Cyprus': 'CYP',
    'Bucharest': 'BUC',
    'Vienna': 'AUS',
}

# CONFIG CHANGE HERE or use -D
#
datalake = "/Users/acute"
db = 'acute'
convert_cmd = 'bdt'


def process_one(fname, action):
    """
    If given a parquet file, convert it into csv and import it.

    :param site: site name.
    :param fname: filename.
    :param action: do we do something or just print?
    :return: converted filename.
    """
    # Deduct site name
    #
    site = find_site(fname)
    if site is None:
        return ''

    ext = Path(fname).suffix
    if ext == '.parquet':
        new = Path(fname).with_suffix('.csv')
        cmd = f"{convert_cmd} convert -s {fname} {new}"
        if action:
            os.system(cmd)
        else:
            print(cmd)
        fname = new
    elif ext != '.csv':
        return None

    # Now do the import, `fname` is a csv file in any case
    #
    ch_cmd = f"clickhouse -d {db} -q \"INSERT INTO airplanes_raw FORMAT Parquet\""
    str = f"/bin/cat {fname} | {ch_cmd}"
    os.system(str)

    # Now we need to fix the `site` column.
    #
    q = f"ALTER TABLE airplanes_raw UPDATE site = {site} WHERE site IS NULL"
    str = f"clickhouse -d {db} -q '{q}'"
    os.system(str)
    return fname


def find_site(fname):
    """
    Return the site shortname deducted from the filename.

    :param fname: full pathname.
    :return: short name.
    """
    fc = re.search(r'^(?P<site>.*?)_(?P<year>\d+)-(?P<month>\d+)-(\d+).parquet$', fname)
    if fc is None:
        return fc
    site: str | Any = fc.group('site')
    return sites[site]


def walk_dir(path, action):
    """
    Explore a given directory tree.

    :param site: site name we pass down for `process_one`
    :param path: base directory.
    :param action: propagate whether we take action or not.
    :return: nothing
    """
    for root, dirs, files in os.walk(path):
        print(f"into {root}")

        # Breadth-first traversal
        #
        for dir in dirs:
            walk_dir(os.path.join(root, dir), action)

        # Now do stuff
        #
        for file in files:
            r = process_one(file, action)
            if r is None:
                print(f"{file} skipped.")


parser = argparse.ArgumentParser(
    prog='import-adsb',
    description='Import ADS-B data into CH.')

parser.add_argument('--datalake', '-D', help='Datalake is here.')
parser.add_argument('--dry-run', '-n', action='store_true', help="Do not actually move the file.")
parser.add_argument('files', nargs='*', help='List of files or directories.')
args = parser.parse_args()

importdir = f"{datalake}/import"
datadir = f"{datalake}/data"
bindir = f"{datalake}/bin"

if args.dry_run:
    action = False
else:
    action = True

files = args.files
for file in files:
    # We have a directory
    #
    if os.path.isdir(file):
        print(f"Exploring {file}")
        walk_dir(file, action)
    else:
        print(f"Just {file}")
        r = process_one(file, action)
        if r is None:
            print(f"{file} skipped.")

