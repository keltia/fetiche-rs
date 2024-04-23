#! /usr/bin/env python3
#
"""
This utility takes a filename or a directory.  If the former, import the given file and if the latter
all parquets files in the tree.
"""

import argparse

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
    }

# CONFIG
#
ch_cmd = 'clickhouse -q "INSERT INTO airplanes_raw FORMAT Parquet'

parser = argparse.ArgumentParser(
    prog='import-adsb',
    description='Import ADS-B data into CH.')

parser.add_argument('--datalake', '-D', help='Datalake is here.')
parser.add_argument('--site', '-S', help="Site")
args = parser.parse_args()

