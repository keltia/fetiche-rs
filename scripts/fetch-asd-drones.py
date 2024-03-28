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
from datetime import datetime, timedelta

# CONFIG CHANGE HERE or use -D
#
datalake = "/Users/acute"
importdir = "/import"
datadir = "/data"
bindir = "/bin"
cmd = "acutectl"


def fetch_files(day):
    os.system(f'')
    os.system(f'/bin/ls -lF {list}')


# Setup arguments
#
parser = argparse.ArgumentParser(
    prog='fetch-asd-drones',
    description='Fetch the last dataset for drones from ASD API.')

parser.add_argument('--datalake', '-D', help='Datalake is here.')
parser.add_argument('--yesterday', '-Y', help='Get data for yesterday.')
parser.add_argument('--keep', '-K', action='store_true', help="Do not delete files after download.")
args = parser.parse_args()

if args.datalake:
    datalake = args.datalake

os.chdir(f'{datalake}{importdir}')

current = datetime.now()
if args.yesterday:
    day = current - timedelta(days=1)
else:
    day = current

fetch_files(day)
