#! /usr/bin/env python3
#
"""
Export all encounters (or the summary).

usage: export-encounters [-h] [--datalake DATALAKE] [--dry-run] [--summary] [--output OUTPUT]

Import ADS-B data into CH.

options:
  -h, --help            show this help message and exit
  --datalake DATALAKE, -D DATALAKE
                        Datalake is here.
  --dry-run, -n         Just show what would happen.
  --summary, -S         Export summary with MIN distances.
  --output OUTPUT, -o OUTPUT
                        Export to a file.
"""
import argparse
import logging
import os
import sys

from datetime import datetime
from pathlib import Path
from subprocess import run

# CONFIG CHANGE HERE or use -D
#
datalake = "/acute"
user = 'acute'
db = 'acute'
url = 'reku.eurocontrol.fr:9000'
export_cmd = "process-data export distances"

# Import DB data from env.
#
user = os.getenv('CLICKHOUSE_USER') or user
pwd = os.getenv('CLICKHOUSE_PASSWD')
name = os.getenv('CLICKHOUSE_DB') or db
url = os.getenv('KLICKHOUSE_URL') or url


def export_encounters(want_summary, action):
    """
    Does the export of data.

    :param want_summary:
    :param action:
    :return:
    """
    fname = f"encounters"
    add_opt = ""
    if want_summary:
        logging.info(f"Exporting summary encounters for {fname}")
        fname = f"{fname}-summary"
        add_opt = "-S"
    else:
        logging.info(f"Exporting all encounters for {fname}")
    fname = os.path.join(outputdir, Path(fname).with_suffix('.csv'))
    output = f"-o {fname}"
    cmd = f"{export_cmd} {add_opt} {output}"
    logging.info(f"Exporting encounters into {fname}")
    if action:
        ret = run(cmd, shell=True, capture_output=True)
        if ret.returncode != 0:
            logging.error("error", "(", fname, "): ", ret.stderr)
            print("error: ", ret.stderr, file=sys.stderr)
    else:
        print(f"Running {cmd}")
    return fname


parser = argparse.ArgumentParser(
    prog='export-encounters',
    description='Export encounters data from CH.')

parser.add_argument('--datalake', '-D', help='Datalake is here.')
parser.add_argument('--dry-run', '-n', action='store_true', help="Just show what would happen.")
parser.add_argument('--summary', '-S', action='store_true', help="Export summary with MIN distances.")
parser.add_argument('--output-dir', '-d', help="Export to this directory.")
args = parser.parse_args()

if args.datalake is not None:
    datalake = args.datalake

importdir = f"{datalake}/import"
datadir = f"{datalake}/data/adsb"
bindir = f"{datalake}/bin"
logdir = f"{datalake}/var/log"
outputdir = f"{datalake}/encounters"

date = datetime.now().strftime('%Y%m%d')
logfile = f"{logdir}/export-encounters-{date}.log"
logging.basicConfig(filemode='a', filename=logfile, level=logging.INFO, datefmt="%H:%M:%S",
                    format='%(asctime)s - %(levelname)s: %(message)s')
logging.info("Starting")

if args.output_dir is not None:
    outputdir = args.output_dir

if args.dry_run:
    action = False
else:
    action = True

if args.summary:
    logging.info("Export only summary.")
    summary = True
else:
    summary = False

fname = export_encounters(summary, action)
print(f"Exported to {fname}")
