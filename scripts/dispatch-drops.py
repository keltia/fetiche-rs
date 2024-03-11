#! /usr/bin/env python3
#
"""
Given a list of files on the command line, parse the filename and dispatch each file where it belongs:

i.e. `Brussels_2024-02-14.parquet` should end up in `adsb/site=BRU/year=2024/month=02`.

This is using Hive partitioning.

usage: dispatch-drops [-h] [--datalake DATALAKE] [--drones] [--dry-run] [files ...]

Move each file in the right Hive directory for the given day.

positional arguments:
  files                List of files.

options:
  -h, --help           show this help message and exit
  --datalake DATALAKE  Datalake is here.
  --drones             This is drone data.
  --dry-run, -n        Do not actually move the file.
"""
import argparse
import re
from pathlib import Path

# CONFIG CHANGE HERE or use -D
#
datalake = "/Users/Acute/data"

# Our current sites
#
sites = {'Brussels': 'BRU',
         'Luxembourg': 'LUX',
         'Bordeaux': 'BDX',
         'Bretigny': 'BRE',
         'Belfast': 'BEL',
         'Cyprus': 'CYP',
         'London': 'LON',
         'Gatwick': 'LON'
         }


def move_one(fn, ftype, action):
    """
    Move one file into the Hve tree.

    :param fn: filename
    :param ftype: type of pat, drones or adsb
    :param action: true does move the file
    :return: nothing
    """
    fname = Path(fn).name
    fc = re.search(r'^(?P<site>.*?)_(?P<year>\d+)-(?P<month>\d+)-(\d+).parquet$', fname)
    if fc is not None:
        site = fc.group('site')

        # Fetch the short name
        #
        try:
            site = sites[site]
        except KeyError:
            print(f'Unknown site {site}')
            return

        year = fc.group('year')
        month = fc.group('month')

        # Create target
        #
        ourdir = f"{datalake}/{ftype}/site={site}/year={year}/month={month}"
        final = Path(ourdir) / fname
        print(f"Moving {fn} into {final}")

        if action:
            Path(fn).rename(final)
    else:
        print(f'Bad file pattern {fn}')


# Setup arguments
#
parser = argparse.ArgumentParser(
    prog='dispatch-drops',
    description='Move each file in the right Hive directory for the given day.')

parser.add_argument('--datalake', help='Datalake is here.')
parser.add_argument('--drones', action='store_true', help='This is drone data.')
parser.add_argument('--dry-run', '-n', action='store_true', help="Do not actually move the file.")
parser.add_argument('files', nargs='*', help='List of files.')
args = parser.parse_args()

if args.datalake:
    datalake = args.datalake

if args.drones:
    ftype = "drones"
else:
    ftype = "adsb"

if args.dry_run:
    action = False
else:
    action = True

files = args.files
for file in files:
    move_one(file, ftype, action)
