#! /usr/bin/env python3
#
"""
This is for importing drone data into Clickhouse.

This utility takes a filename or a directory.  If the former, import the given file and if the latter
all parquets files in the tree.

XXX this is specific to the macOS version of the client, invoked as `clickhouse client` and not
`clickhouse-client` or `clickhouse-local` like in the other versions.

XXX You must have `bdt(1)`  somewhere in the `PATH`
"""

import argparse
import os
import re
import sys
import tempfile
from pathlib import Path
from typing import Any

# CONFIG CHANGE HERE or use -D
#
datalake = "/Users/acute"
db = 'acute'
convert_cmd = 'bdt'
clickhouse = 'clickhouse-client'
if sys.platform.startswith('darwin'):
    clickhouse = 'clickhouse client'


def process_one(dir, fname, action):
    """
    If given a parquet file, convert it into csv and import it.

    :param dir directory part of the file path.
    :param fname: filename.
    :param action: do we do something or just print?
    :return: converted filename.
    """
    print(f">> Looking at {os.path.join(root, fname)}")

    ext = Path(fname).suffix

    if ext == '.parquet':
        csv = Path(fname).with_suffix('.csv')
        if os.path.exists(os.path.join(dir, csv)):
            print(f"Warning: both parquet & csv exist for {fname}, ignoring parquet.")
            fname = csv
        else:
            full = os.path.join(dir, fname)
            new = tempfile.NamedTemporaryFile(suffix='.csv').name
            cmd = f"{convert_cmd} convert -s {full} {new}"
            if action:
                os.system(cmd)
            else:
                print(cmd)
            fname = new

    # Now do the import, `fname` is a csv file in any case
    #
    host = os.getenv('CLICKHOUSE_HOST')
    user = os.getenv('CLICKHOUSE_USER')
    pwd = os.getenv('CLICKHOUSE_PASSWD')

    ch_cmd = f"{clickhouse} -h {host} -u {user} -d {db} --password {pwd} -q \"INSERT INTO drones_raw FORMAT Csv\""
    cmd = f"/bin/cat {os.path.join(dir, fname)} | {ch_cmd}"
    if action:
        os.system(cmd)
    else:
        print(f"Running {cmd}")

    return fname


parser = argparse.ArgumentParser(
    prog='import-adsb',
    description='Import drone data into CH.')

parser.add_argument('--datalake', '-D', help='Datalake is here.')
parser.add_argument('--dry-run', '-n', action='store_true', help="Do not actually move the file.")
parser.add_argument('files', nargs='*', help='List of files or directories.')
args = parser.parse_args()

importdir = f"{datalake}/import"
datadir = f"{datalake}/data/drones"
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
        for root, dirs, files in os.walk(file, topdown=True):
            print(f"into {root}")

            # Now do stuff, look at parquet/csv only
            #
            for f in files:
                if Path(f).suffix != '.parquet' and Path(f).suffix != '.csv':
                    continue

                r = process_one(root, f, action)
                if r is None:
                    print(f"{f} skipped.")
    else:
        print(f"Just {file}")
        root = Path(file).root
        r = process_one(root, file, action)
        if r is None:
            print(f"{file} skipped.")

