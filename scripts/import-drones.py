#! /usr/bin/env python3
#
"""
This is for importing drone data into Clickhouse.

This utility takes a filename or a directory.  If the former, import the given file and if the latter
all parquets files in the tree.

XXX You must have `bdt(1)`  somewhere in the `PATH`
"""

import argparse
from datetime import datetime
import logging
import os
import sys
import tempfile
from pathlib import Path
from subprocess import call

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
    logging.info(f"Looking at {os.path.join(root, fname)}")

    ext = Path(fname).suffix

    if ext == '.parquet':
        csv = Path(fname).with_suffix('.csv')
        if os.path.exists(os.path.join(dir, csv)):
            logging.warning(f"both parquet & csv exist for {fname}, ignoring parquet.")
            fname = csv
        else:
            full = os.path.join(dir, fname)
            new = tempfile.NamedTemporaryFile(suffix='.csv').name
            cmd = f"{convert_cmd} convert -s {full} {new}"
            logging.info(f"converting {cmd}")
            if action:
                try:
                    call(cmd, shell=True)
                except OSError as err:
                    print("error: ", err, file=sys.stderr)
                    logging.error("error: ", err)
            else:
                print(cmd)
            fname = new

    # Now do the import, `fname` is a csv file in any case
    #
    host = os.getenv('CLICKHOUSE_HOST')
    user = os.getenv('CLICKHOUSE_USER')
    pwd = os.getenv('CLICKHOUSE_PASSWD')
    dbn = os.getenv('CLICKHOUSE_DB') or db

    ch_cmd = f"{clickhouse} -h {host} -u {user} -d {dbn} --password {pwd} -q \"INSERT INTO drones_raw FORMAT Csv\""
    cmd = f"/bin/tail -n +2 {os.path.join(dir, fname)} | {ch_cmd}"
    logging.info(f"{cmd}")
    if action:
        try:
            call(cmd, shell=True)
        except OSError as err:
            print("error: ", err, file=sys.stderr)
            logging.error("error: ", err)
    else:
        print(f"Running {cmd}")
    logging.info("Import done.")
    return fname


parser = argparse.ArgumentParser(
    prog='import-drones',
    description='Import drone data into CH.')

parser.add_argument('--datalake', '-D', help='Datalake is here.')
parser.add_argument('--dry-run', '-n', action='store_true', help="Do not actually move the file.")
parser.add_argument('files', nargs='*', help='List of files or directories.')
args = parser.parse_args()

if args.datalake is not None:
    datalake = args.datalake

importdir = f"{datalake}/import"
datadir = f"{datalake}/data/drones"
bindir = f"{datalake}/bin"
logdir = f"{datalake}/var/log"

date = datetime.now().strftime('%Y%m%d')
logfile = f"{logdir}/import-drones-{date}.log"
logging.basicConfig(filemode='a', filename=logfile, level=logging.INFO, datefmt="%H:%M:%S",
                    format='%(asctime)s - %(levelname)s: %(message)s')
logging.info("Starting")

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
                if not name.startswith('drones-'):
                    logging.warning(f"{f} ignored.")
                    continue

                r = process_one(root, f, action)
                if r is None:
                    logging.warning(f"{f} skipped.")
    else:
        logging.info(f"Just {file}")
        root = Path(file).root
        r = process_one(root, file, action)
        if r is None:
            logging.warning(f"{file} skipped.")
