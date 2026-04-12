#! /usr/bin/env python3
#
"""
This is for importing ADS-B data into Clickhouse.

This utility takes a filename or a directory.  If the former, import the given file and if the latter
all parquets files in the tree.

XXX this is specific to the macOS version of the client, invoked as `clickhouse client` and not
`clickhouse-client` or `clickhouse-local` like in the other versions.

XXX You must have `bdt(1)` and `qsvlite(1)` somewhere in the `PATH`
"""

import argparse
import csv
import logging
import os
import re
import tempfile
from datetime import datetime
from pathlib import Path
from subprocess import run
from typing import Any

import sys
import time

# CONFIG CHANGE HERE or use -D
#
datalake = "/acute"
db = 'acute'
table = f"{db}.airplanes_raw"
convert_cmd = 'bdt'
csv_cmd = 'qsvlite'
delete = False
site_id = 0

clickhouse = 'clickhouse-client'
if sys.platform.startswith('darwin'):
    clickhouse = 'clickhouse client'
    csv_cmd = 'qsv'


# Import sites.csv
#
def load_sites(path):
    """
    Load sites data from CSV file.

    :param path: Base directory path containing sites.csv
    :return: Dict of sites data
    """
    sites_path = Path(path) / "sites.csv"
    if not sites_path.exists():
        logging.error(f"Sites file {sites_path} not found")
        return {}

    try:
        with open(sites_path) as f:
            reader = csv.DictReader(f)
            sites_data = {}
            for row in reader:
                sites_data[row['basename']] = int(row['id'])
            return sites_data
    except Exception as e:
        logging.error(f"Error loading sites.csv: {e}")
        return {}


# Import DB data from env.
#
host = os.getenv('CLICKHOUSE_HOST')
user = os.getenv('CLICKHOUSE_USER')
dbn = os.getenv('CLICKHOUSE_DB') or db


def process_one(dir_path, fname, action):
    """
    If given a parquet file, convert it into csv and import it.

    :param dir_path directory part of the file path.
    :param fname: filename.
    :param action: do we do something or just print?
    :return: converted filename.
    """
    print(f"file={fname}")

    # Deduct site name
    #
    site = find_site(fname)
    if site is None or site == 0:
        logging.error(f"site extracted from {fname} does not exist, skipping.")
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
            ret = run(cmd, shell=True, capture_output=True)
            if ret.returncode != 0:
                logging.error("error: ", ret.stderr)
                print("error: ", ret.stderr, file=sys.stderr)
                return fname
        else:
            print(f"cmd={cmd} -> {fname}")

    if ext == '.parquet':
        logging.info(f"found parquet file {fname}")
        csv = Path(fname).with_suffix('.csv')
        if os.path.exists(os.path.join(dir_path, csv)):
            logging.warning(f"Warning: both parquet & csv exist for {fname}, ignoring parquet.")
            fname = csv
        else:
            full = os.path.join(dir_path, fname)
            with tempfile.NamedTemporaryFile(suffix='.csv', delete=False) as tmp:
                new = tmp.name

            cmd = f"{convert_cmd} convert -s {full} {new}"
            logging.info(f"{cmd}")
            if action:
                ret = run(cmd, shell=True, capture_output=True)
                if ret.returncode != 0:
                    logging.error("error", "(", fname, "): ", ret.stderr)
                    print("error: ", ret.stderr, file=sys.stderr)
                    return fname
            else:
                print(f"Running {cmd}")
            fname = new

    # Now, we have a csv file, we need to add the new column based on the site id
    # and import.

    logging.info(f"Adding column Site with {site} and importing from {fname}…")
    ch_cmd = f"{clickhouse} -h {host} -u {user} --password $CLICKHOUSE_PASSWD -q \"INSERT INTO {table} FORMAT CsvWithNames\""
    cmd = f"{csv_cmd} enum -c Site --constant {site} {fname} | {ch_cmd}"

    logging.info(f"cmd={cmd}")
    if action:
        ret = run(cmd, shell=True, capture_output=True)
        if ret.returncode != 0:
            logging.error("error", "(", os.path.join(dir_path, fname), "): ", ret.stderr)
            print("error: ", ret.stderr, file=sys.stderr)
            return fname
    else:
        print(f"Running {cmd}")
    logging.info(f"insert from {fname} into {table} done.")

    # Cleanup
    #
    if delete:
        if action:
            os.remove(fname)
        logging.info("delete done.")

    return fname


def find_site(fname):
    """
    Return the site ID deducted from the filename.

    :param fname: full pathname.
    :return: short name.
    """
    name = Path(fname).name
    fc = re.search(r'^(?P<site>.*?)([0-9]*)_(?P<year>\d+)-(?P<month>\d+)-(\d+).', name)
    if fc is None:
        return fc

    # If we have defined a site id, use it.
    #
    if site_id > 0:
        return site_id
    site: str | Any = fc.group('site')
    return sites[site]


def test_find_site():
    """Test find_site function"""
    # Test valid site name
    assert find_site("Bretigny_2023-12-01.parquet") == 1
    assert find_site("Luxembourg_2023-12-01.csv") == 3

    # Test invalid filename format
    assert find_site("invalid_filename.txt") is None

    # Test unknown site
    assert find_site("Unknown_2023-12-01.parquet") is None

    # Test with numbered site variant
    assert find_site("Vienna2_2023-12-01.parquet") == 10


parser = argparse.ArgumentParser(
    prog='import-adsb',
    description='Import ADS-B data into CH.')

parser.add_argument('--datalake', '-D', help='Datalake is here.')
parser.add_argument('--dry-run', '-n', action='store_true', help="Just show what would happen.")
parser.add_argument('--delete', '-d', action='store_true', help="Delete final file.")
parser.add_argument('--interval', '-i', type=int, help='Interval between imports.')
parser.add_argument('--no-delay', '-N', action='store_true', help='Do not add delay between imports.')
parser.add_argument('--site', '-s', help='Override site id.')
parser.add_argument('--table', '-T', help="Name of the table to import into.")
parser.add_argument('files', nargs='*', help='List of files or directories.')
args = parser.parse_args()

if args.datalake is not None:
    datalake = args.datalake

importdir = f"{datalake}/import"
datadir = f"{datalake}/data/adsb"
bindir = f"{datalake}/bin"
logdir = f"{datalake}/var/log"
filesdir = f"{datalake}/files"

sites = load_sites(filesdir)

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

if args.site is not None:
    site_id = int(args.site)
    logging.info(f"Force site id {site_id}")

if args.table is not None:
    table = args.table
    logging.info(f"Importing into {table}.")

# Default interval between imports is 5s
#
if args.interval is None:
    interval = 5
else:
    interval = args.interval

if not args.no_delay:
    logging.info(f"Delay is {interval}s")

files = args.files
for file in files:
    # We have a directory
    #
    if os.path.isdir(file):
        print(f"Exploring {file}")
        logging.info(f"Inside {file}")
        for root, dirs, file_list in os.walk(file, topdown=True):
            logging.info(f"into {root}")

            # Now do stuff, look at parquet/csv only
            #
            for f in file_list:
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

                if not args.no_delay:
                    time.sleep(interval)
    else:
        logging.info(f"file={file}")
        root = Path(file).parent
        r = process_one(root, file, action)
        if r is None:
            logging.warning(f"{file} skipped.")
