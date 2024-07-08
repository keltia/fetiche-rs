#! /usr/bin/env python3
#
"""
This fetch the latest ASD drone drop from ASD API (default is yesterday)

usage: fetch-asd-drones [-h] [-D DATALAKE]

Fetch the drone data from ASD API for a day (default is yesterday)

options:
  -h, --help            show this help message and exit
  --datalake DATALAKE, -D DATALAKE
                        Datalake is here.

"""

import argparse
import os
from datetime import datetime, timedelta, timezone

# CONFIG CHANGE HERE or use -D
#
datalake = "/Users/acute"
cmd = "acutectl"


def fetch_files(site, output):
    str = f"{cmd} fetch -o {output} {site} yesterday"
    print(f"cmd={str}")
    os.system(str)
    os.system('/bin/ls -lF')


# Setup arguments
#
parser = argparse.ArgumentParser(
    prog='fetch-asd-drones',
    description='Fetch the last dataset for drones from ASD API.')

parser.add_argument('--site', '-S', help='Use this site.')
parser.add_argument('--datalake', '-D', help='Datalake is here.')
parser.add_argument('--keep', '-K', action='store_true', help="Do not delete files after download.")
args = parser.parse_args()

site = ''
if args.datalake:
    datalake = args.datalake

if args.site is None:
    print("You must specify a site.")
    exit(1)

importdir = f"{datalake}/import"
datadir = f"{datalake}/data"
bindir = f"{datalake}/bin"

os.chdir(importdir)

day = datetime.now(timezone.utc) - timedelta(days=1)
output = f"drones-{day.year}-{day.month:02}-{day.day:02}.parquet"

fetch_files(args.site, output)
