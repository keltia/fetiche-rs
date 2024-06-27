#! /usr/bin/env python3
#
"""
This is for importing ADS-B data into Clickhouse.

This utility takes a filename or a directory.  If the former, import the given file and if the latter
all parquets files in the tree.

XXX this is specific to the macOS version of the client, invoked as `clickhouse client` and not
`clickhouse-client` or `clickhouse-local` like in the other versions.

XXX You must have `bdt(1)`  somewhere in the `PATH`
"""

import argparse
from datetime import datetime
import logging
import os
import re
import sys
import tempfile
from pathlib import Path
from subprocess import call
from typing import Any

# Does the mapping between the site basename and its ID.  Not worth using SQL for that.
#
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
    'Vienna2': 10,
}

# CONFIG CHANGE HERE or use -D
#
datalake = "/Users/acute"
db = 'acute'
convert_cmd = 'bdt'
clickhouse = 'clickhouse-client'
if sys.platform.startswith('darwin'):
    clickhouse = 'clickhouse client'

delete = False

def process_one(dir, fname, action):
    """
    If given a parquet file, convert it into csv and import it.

    :param dir directory part of the file path.
    :param fname: filename.
    :param action: do we do something or just print?
    :return: converted filename.
    """

    # Deduct site name
    #
    site = find_site(fname)
    if site is None:
        logging.error(f"{site} not found.")
        return ''
    logging.info(f"site={site}")

    ext = Path(fname).suffix

    # .csv.gz ?
    #
    if ext == '.gz':
        cmd = f"gunzip {fname}"
        fname = Path(fname).stem
        ext = Path(fname).suffix
        logging.info(f"{cmd} -> {fname}")
        if action:
            try:
                call(cmd, shell=True)
            except OSError as err:
                print("error: ", err, file=sys.stderr)
        else:
            print(f"cmd={cmd} -> {fname}")

    if ext == '.parquet':
        logging.info(f"found parquet file {fname}")
        csv = Path(fname).with_suffix('.csv')
        if os.path.exists(os.path.join(dir, csv)):
            logging.warning(f"Warning: both parquet & csv exist for {fname}, ignoring parquet.")
            fname = csv
        else:
            full = os.path.join(dir, fname)
            new = tempfile.NamedTemporaryFile(suffix='.csv').name
            cmd = f"{convert_cmd} convert -s {full} {new}"
            logging.info(f"{cmd}")
            if action:
                try:
                    call(cmd, shell=True)
                except OSError as err:
                    print("error: ", err, file=sys.stderr)
            else:
                print(cmd)
            fname = new

    # Now do the import, `fname` is a csv file in any case
    #
    host = os.getenv('CLICKHOUSE_HOST')
    user = os.getenv('CLICKHOUSE_USER')
    pwd = os.getenv('CLICKHOUSE_PASSWD')
    dbn = os.getenv('CLICKHOUSE_DB') or db

    ch_cmd = f"{clickhouse} -h {host} -u {user} -d {dbn} --password {pwd} -q \"INSERT INTO airplanes_raw FORMAT Csv\""
    cmd = f"/bin/cat {os.path.join(dir, fname)} | {ch_cmd}"
    logging.info(f"cmd={cmd}")
    if action:
        try:
            call(cmd, shell=True)
        except OSError as err:
            print("error: ", err, file=sys.stderr)
            logging.error("error: ", err)
    else:
        print(f"Running {cmd}")
    logging.info("insert done.")

    # Now we need to fix the `site` column.
    #
    q = f"ALTER TABLE acute.airplanes_raw UPDATE site = '{site}' WHERE site = 0"
    cmd = f"{clickhouse} -h {host} -u {user} -d {db} --password {pwd} -q '{q}'"
    logging.info(cmd)
    if action:
        try:
            call(cmd, shell=True)
        except OSError as err:
            print("error: ", err, file=sys.stderr)
            logging.error("error: ", err)
    else:
        print(f"cmd={cmd}")
    logging.info(f"update for site {site} done.")

    # Now delete if requested
    #
    if delete:
        logging.info("delete done.")
        os.remove(fname)
    return fname


def find_site(fname):
    """
    Return the site shortname deducted from the filename.

    :param fname: full pathname.
    :return: short name.
    """
    name = Path(fname).name
    fc = re.search(r'^(?P<site>.*?)_(?P<year>\d+)-(?P<month>\d+)-(\d+).', name)
    if fc is None:
        return fc
    site: str | Any = fc.group('site')
    return sites[site]


parser = argparse.ArgumentParser(
    prog='import-adsb',
    description='Import ADS-B data into CH.')

parser.add_argument('--datalake', '-D', help='Datalake is here.')
parser.add_argument('--dry-run', '-n', action='store_true', help="Just show what would happen.")
parser.add_argument('--delete', '-d', action='store_true', help="Delete final file.")
parser.add_argument('files', nargs='*', help='List of files or directories.')
args = parser.parse_args()

if args.datalake is not None:
    datalake = args.datalake

importdir = f"{datalake}/import"
datadir = f"{datalake}/data/adsb"
bindir = f"{datalake}/bin"
logdir = f"{datalake}/var/log"

date = datetime.now().strftime('%Y%m%d')
logfile = f"{logdir}/import-adsb-{date}.log"
logging.basicConfig(filemode='a', filename=logfile, level=logging.INFO, datefmt="%H:%M:%S",
                    format='%(asctime)s - %(levelname)s: %(message)s')
logging.info("Starting")

if args.dry_run:
    action = False
else:
    action = True

if args.delete:
    delete = True

files = args.files
for file in files:
    # We have a directory
    #
    if os.path.isdir(file):
        print(f"Exploring {file}")
        logging.info(f"Inside {file}")
        for root, dirs, files in os.walk(file, topdown=True):
            logging.info(f"into {root}")

            # Now do stuff, look at parquet/csv only
            #
            for f in files:
                if Path(f).suffix != '.parquet' and Path(f).suffix != '.csv':
                    logging.warning(f"{f} ignored.")
                    continue

                # Ignore non drones-related files
                #
                name = Path(f).stem
                if name.startswith('drones-'):
                    logging.warning(f"{f} ignored.")
                    continue

                r = process_one(root, f, action)
                if r is None:
                    logging.warning(f"{f} skipped.")
    else:
        logging.info(f"file={file}")
        root = Path(file).root
        r = process_one(root, file, action)
        if r is None:
            logging.warning(f"{file} skipped.")

