#! /usr/bin/env python3
#
# Short pipeline for importing/archiving Avionix data.
#
# If `.` is specified as filename, try to compute last day as YYYYMMDD and use it as `BNAME`
#
# NOTE CLICKHOUSE_* variables MUST be defined at run-time.
#
import argparse
import logging
import os
import sys
import tempfile
import time
from datetime import datetime, timedelta, timezone
from os.path import exists
from pathlib import Path
from subprocess import run


# CONFIG CHANGE HERE or use -D
#
datalake = "/acute"
db = 'acute'
convert_cmd = f"{datalake}/bin/convert-from-json"
action = True
delete = False
clickhouse = 'clickhouse-client'
if sys.platform.startswith('darwin'):
    clickhouse = 'clickhouse client'


# Conversion part
#
# Take the json file and generate both csv and parquet.
#
# Import one
#
def process_one(path, action):
    logging.info(f"Processing {path}")

    # Check file exists
    #
    if not exists(path):
        return None

    today = datetime.now(timezone.utc)
    month = f"{today:%m}"
    year = f"{today:%Y}"
    basename = Path(path).stem
    final = Path(path).with_suffix('.parquet')

    logging.info(f"Processing {path}")
    logging.info(f"Basename is {basename}")

    logging.info(f"Converting into parquet for archival.")
    if action:
        cmd = f"{convert_cmd} -P {path}"
        logging.info(f"Running {cmd}")
        run(cmd, shell=True)
        if not exists(final):
            logging.error(f"{final} does not exist.")
            return None

    logging.info(f"Moving to {tree}")
    tree = f"{datalake}/drones/year={year}/month={month}"
    os.makedirs(tree, 0o775, exist_ok=True)

    if action:
        cmd = f"{bindir}/import-avionix.py -D {datalake} -d {path}"
        logging.info(f"Running {cmd}")
        run(cmd, shell=True)

echo "Cwd is ${PWD}"
echo "Basename is ${BNAME}"
#
MONTH=$(date +%m)
YEAR=$(date +%Y)

# First step — convert into csv & parquet
#
# FIXME columns order will be sorted, this is a datafusion BUG
#
echo "Convert to parquet"
sort -u "${BNAME}.json" > "${BNAME}s.json" && \
	bdt convert -s "${BNAME}s.json" "${BNAME}.parquet"

# Second step — create our archive tree
#
echo "Create dir tree"
[[ ! -d "${DATADIR}/year=${YEAR}/month=${MONTH}" ]] && \
	mkdir -p "${DATADIR}/year=${YEAR}/month=${MONTH}"

# Third step a  — move
echo "Move files."
mv "${BNAME}.parquet" "${DATADIR}/year=${YEAR}/month=${MONTH}/"

# Third step b  — import
#
#${BASEDIR}/bin/import-avionix.py -D ${BASEDIR} -d "${BNAME}.csv" && \
#	rm -f "${BNAME}s.json" "${BNAME}.json" "${BNAME}.csv"
echo "Insert into CH."
cat "${BNAME}s.json" | clickhouse-client -h $CLICKHOUSE_HOST -u $CLICKHOUSE_USER \
  --password $CLICKHOUSE_PASSWD -d $CLICKHOUSE_DB \
  -q 'INSERT INTO avionix_raw FORMAT JSONEachRow' && \
  rm -f "${BNAME}s.json" "${BNAME}.json"
[[ $? == 0 ]] && echo "End."





# Check arguments
#
parser = argparse.ArgumentParser(
    prog='avionix-pipeline',
    description='Import Avionix data into CH.')

parser.add_argument('--datalake', '-D', help='Datalake is here.')
parser.add_argument('--dry-run', '-n', action='store_true', help="Just show what would happen.")
parser.add_argument('--delete', '-d', action='store_true', help="Delete final file.")
parser.add_argument('--interval', '-i', type=int, help='Interval between imports.')
parser.add_argument('--no-delay', '-N', action='store_true', help='Do not add delay between imports.')
parser.add_argument('files', nargs='*', help='List of files or directories.')
args = parser.parse_args()

if args.datalake is not None:
    datalake = args.datalake

bindir = f"{datalake}/bin"
datadir = f"{datalake}/data/avionix"
logdir = f"{datalake}/var/log"

date = datetime.now().strftime('%Y%m%d')
logfile = f"{logdir}/avionix-pipeline-{date}.log"
logging.basicConfig(filemode='a', filename=logfile, level=logging.INFO, datefmt="%H:%M:%S",
                    format='%(asctime)s - %(levelname)s: %(message)s')
logging.info("Starting")

# Import DB data from env.
#
host = os.getenv('CLICKHOUSE_HOST')
user = os.getenv('CLICKHOUSE_USER')
pwd = os.getenv('CLICKHOUSE_PASSWD')
dbn = os.getenv('CLICKHOUSE_DB') or db

if args.dry_run:
    action = False
else:
    action = True

if args.delete:
    delete = True

# Default interval between imports is 5s
#
if args.interval is None:
    interval = 5
else:
    interval = args.interval

if args.no_delay is None:
    logging.info(f"Delay is {interval}s")

files = args.files
for file in files:
    if file == 'yesterday':
        basename = datetime.now(timezone.utc) - timedelta(days=1)
        file = f"{basename:%Y%m%d}.json"
        r = process_one(file, action)
        if r is None:
            logging.warning(f"{file} skipped.")

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
                if not name.startswith('avionix-'):
                    logging.warning(f"{f} ignored.")
                    continue

                r = process_one(root, f, action)
                if r is None:
                    logging.warning(f"{f} skipped.")

                if args.no_delay is None:
                    time.sleep(interval)
    else:
        logging.info(f"file={file}")
        root = Path(file).root
        r = process_one(root, file, action)
        if r is None:
            logging.warning(f"{file} skipped.")





ARG=$1
if [ x"$ARG" = x"." ]; then
  BNAME=$(date +"%Y%m%d" -d yesterday)
else
  BNAME=$ARG:r
fi

