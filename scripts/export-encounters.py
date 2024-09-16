#! /usr/bin/env python3
#
"""

"""
import argparse
import logging
import os
import sys

from datetime import datetime
from subprocess import run

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
        q1 = " \
CREATE OR REPLACE TABLE airprox_summary \
ENGINE = Memory \
AS ( \
  SELECT \
    en_id, \
    journey, \
    drone_id, \
    min(distance_slant_m) as distance_slant_m \
  FROM \
    airplane_prox \
  GROUP BY \
    en_id,journey,drone_id"
        cmd = f"{clickhouse} -h {host} -u {user} -d {db} --password {pwd} -q '{q}'"
        logging.info(cmd)
        if action:
            ret = run(cmd, shell=True, capture_output=True)
            if ret.returncode != 0:
                logging.error("error", "(", fname, "): ", ret.stderr)
                print("error: ", ret.stderr, file=sys.stderr)

        q2 = (f" \
    SELECT \
    a.en_id, \
    a.site, \
    a.time, \
    a.journey, \
    a.drone_id, \
    a.model, \
    a.drone_lat, \
    a.drone_lon, \
    a.drone_alt_m, \
    a.drone_height_m, \
    a.prox_callsign, \
    a.prox_id, \
    a.prox_lat, \
    a.prox_lon, \
    a.prox_alt_m, \
    a.distance_hor_m, \
    a.distance_vert_m, \
    a.distance_home_m, \
    a.distance_slant_m, \
  FROM \
    airplane_prox AS a JOIN airprox_summary AS s \
    ON \
    s.en_id = a.en_id AND \
    s.journey = a.journey AND \
    s.drone_id = a.drone_id \
  WHERE \
    a.distance_slant_m = s.distance_slant_m \
  ORDER BY time \
  INTO OUTFILE '{fname}' FORMAT CSVWithNames")
    else:
        q = (f" \
    SELECT \
    en_id, \
    site, \
    time, \
    journey, \
    drone_id, \
    model, \
    drone_lat, \
    drone_lon, \
    drone_alt_m, \
    drone_height_m, \
    prox_callsign, \
    prox_id, \
    prox_lat, \
    prox_lon, \
    prox_alt_m, \
    distance_hor_m, \
    distance_vert_m, \
    distance_home_m, \
    distance_slant_m, \
  FROM airplane_prox \
  ORDER BY time \
  INTO OUTFILE '{fname}' FORMAT CSVWithNames")


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
