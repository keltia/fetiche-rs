#! /usr/bin/env python3
#
"""
This utility takes a filename or a directory.  If the former, import the given file and if the latter
all parquets files in the tree.
"""

import argparse
import os
from pathlib import Path

sites = {
    'Bretigny': 1,
    'Luxembourg': 3,
    'Brussels': 4,
    'Belfast': 5,
    'Bordeaux': 6,
    'Gatwick': 7,
    'London': 7,
    'Cyprus': 8,
    'Bucharest': 9,
    'Vienna': 10,
}

# CONFIG CHANGE HERE or use -D
#
datalake = "/Users/acute"
ch_cmd = 'clickhouse -q "INSERT INTO airplanes_raw FORMAT Parquet'
convert_cmd = 'bdt'


def process_one(fname, action):
    ext = Path(fname).suffix
    new = Path(fname).with_suffix('.csv')
    if ext == '.parquet':
        cmd = f"{convert_cmd} convert -s {fname} {new}"
        if action:
            os.system(cmd)
        else:
            print(cmd)
    return new


def walk_dir(path, action):
    for root, dirs, files in os.walk(path):
        print(f"into {root}")
        for file in files:
            process_one(file, action)
        for dir in dirs:
            walk_dir(os.path.join(root, dir), action)


parser = argparse.ArgumentParser(
    prog='import-adsb',
    description='Import ADS-B data into CH.')

parser.add_argument('--datalake', '-D', help='Datalake is here.')
parser.add_argument('--dry-run', '-n', action='store_true', help="Do not actually move the file.")
parser.add_argument('--site', '-S', help="Site")
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
        process_one(file, action)

