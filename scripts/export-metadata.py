#! /usr/bin/env python3
#
"""
This script exports all CSV files corresponding to tables in Clickhouse after some operations,
like update the `sites` or `installations` tables.
"""
import argparse
import logging
import os
import sys
from datetime import datetime
from subprocess import run

# CONFIG CHANGE HERE or use -D
#
datalake = "/Users/acute"
db = 'acute'
clickhouse = 'clickhouse-client'
if sys.platform.startswith('darwin'):
    clickhouse = 'clickhouse client'

# All exportable tables
reqs = {
    'sites': 'select * from {} order by id into outfile \'{}/{}.csv\' truncate format csvwithnames',
    'installations': 'select * from {} order by id into outfile \'{}/{}.csv\' truncate format csvwithnames',
    'deployments': 'select * from {} order by install_id into outfile \'{}/{}.csv\' truncate format csvwithnames',
    'pbi_deployments': 'select * from {} order by installation_id into outfile \'{}/{}.csv\' truncate format csvwithnames',
}


def export_one(tag, action):
    # tag is the name of the table and the final csv file
    #
    host = os.getenv('CLICKHOUSE_HOST')
    user = os.getenv('CLICKHOUSE_USER')
    pwd = os.getenv('CLICKHOUSE_PASSWD')
    dbn = os.getenv('CLICKHOUSE_DB') or db

    req = reqs[tag]
    cmd = f"{clickhouse} -h {host} -u {user} -d {dbn} --password {pwd} -q \"{req.format(tag, filesdir, tag)}\""
    logging.info(f"{cmd}")
    if action:
        ret = run(cmd, shell=True, capture_output=True)
        if ret.returncode != 0:
            logging.error("error: ", ret.stderr)
            print("error: ", ret.stderr, file=sys.stderr)
    else:
	    print(f"{cmd}")
    print(f"Exported {tag} to {filesdir}/{tag}.csv")


parser = argparse.ArgumentParser(
    prog='export-metadata',
    description='Export ACUTE metadata as CSV files.')

parser.add_argument('--datalake', '-D', help='Datalake is here.')
parser.add_argument('--dry-run', '-n', action='store_true', help="Just show what would happen.")
args = parser.parse_args()

if args.datalake is not None:
    datalake = args.datalake

importdir = f"{datalake}/import"
filesdir = f"{datalake}/files"
bindir = f"{datalake}/bin"
logdir = f"{datalake}/var/log"

date = datetime.now().strftime('%Y%m%d')
logfile = f"{logdir}/export-metadata-{date}.log"
logging.basicConfig(filemode='a', filename=logfile, level=logging.INFO, datefmt="%H:%M:%S",
                    format='%(asctime)s - %(levelname)s: %(message)s')
logging.info("Starting")

if args.dry_run:
    action = False
else:
    action = True

for table in reqs.keys():
    export_one(table, action)
