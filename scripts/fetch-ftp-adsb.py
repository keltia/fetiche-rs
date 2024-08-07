#! /usr/bin/env python3
#
"""
This fetch the latest ADS-B drops from ftps.eurocontrol.fr.

usage: fetch-ftp-adsb [-h] [-D DATALAKE] [--keep]

Fetch the last files from the incoming directory on ftps.

options:
  -h, --help            show this help message and exit
  --datalake DATALAKE, -D DATALAKE
                        Datalake is here.
  --keep, -K            Do not delete files after download.

NOTE: you must have a bookmark defined for ftps.eurocontrol.fr, we do not store passwords in scripts, EVER.
"""

import argparse
import os

# CONFIG CHANGE HERE or use -D
#
datalake = "/Users/acute"

# Use the bookmark name
#
site = "ftp_avt"


def fetch_files(list):
    os.system(f'lftp -f {bindir}/fetch-all-adsb.txt')
    os.system(f'/bin/ls -lF {list}')


# Setup arguments
#
parser = argparse.ArgumentParser(
    prog='fetch-ftp-adsb',
    description='Fetch the last files from the incoming directory on ftps.')

parser.add_argument('--datalake', '-D', help='Datalake is here.')
parser.add_argument('--keep', '-K', action='store_true', help="Do not delete files after download.")
args = parser.parse_args()

if args.datalake:
    datalake = args.datalake

importdir = f"{datalake}/import"
datadir = f"{datalake}/data"
bindir = f"{datalake}/bin"

os.chdir(importdir)

fetch_files('*.gz')
