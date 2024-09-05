#! /usr/bin/env python3
#
"""

"""
import argparse
import logging
import os
import sys

from datetime import datetime

# CONFIG CHANGE HERE or use -D
#
datalake = "/acute"
db = 'acute'
clickhouse = 'clickhouse-client'
if sys.platform.startswith('darwin'):
    clickhouse = 'clickhouse client'

# Import DB data from env.
#
host = os.getenv('CLICKHOUSE_HOST')
user = os.getenv('CLICKHOUSE_USER')
pwd = os.getenv('CLICKHOUSE_PASSWD')
dbn = os.getenv('CLICKHOUSE_DB') or db


def export_encounters(want_summary, fname):
    if want_summary:

    else:
        q = f""


parser = argparse.ArgumentParser(
    prog='import-adsb',
    description='Import ADS-B data into CH.')

parser.add_argument('--datalake', '-D', help='Datalake is here.')
parser.add_argument('--dry-run', '-n', action='store_true', help="Just show what would happen.")
parser.add_argument('--summary', '-S', help="Export summary with MIN distances.")
parser.add_argument('--output', '-o', help="Export to a file.")
args = parser.parse_args()

if args.datalake is not None:
    datalake = args.datalake

importdir = f"{datalake}/import"
datadir = f"{datalake}/data/adsb"
bindir = f"{datalake}/bin"
logdir = f"{datalake}/var/log"

date = datetime.now().strftime('%Y%m%d')
logfile = f"{logdir}/export-encounters-{date}.log"
logging.basicConfig(filemode='a', filename=logfile, level=logging.INFO, datefmt="%H:%M:%S",
                    format='%(asctime)s - %(levelname)s: %(message)s')
logging.info("Starting")

if args.dry_run:
    action = False
else:
    action = True

if args.summary:
    logging.info("Export only summary.")
    summary = True

if args.output is None:
    output = "export-excounters.csv"

export_encounters(summary, output)
