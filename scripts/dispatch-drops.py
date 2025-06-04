#! /usr/bin/env python3
#
"""
Given a list of files on the command line, parse the filename and dispatch each file where it belongs:

i.e. `Brussels_2024-02-14.parquet` should end up in `adsb/site=BRU/year=2024/month=02`.

This is using Hive partitioning.

usage: dispatch-drops [-h] [--datalake DIR] [--drones] [--dry-run] [files ...]

Move each file in the right Hive directory for the given day.

positional arguments:
  files                List of files or directories.

options:
  -h, --help           show this help message and exit
  --datalake DATALAKE  Datalake is here.
  --drones             This is drone data.
  --dry-run, -n        Do not actually move the file.
"""
import argparse
import csv
import logging
import os
import re
from datetime import datetime
from pathlib import Path

# CONFIG CHANGE HERE or use -D
#
datalake = "/acute"


# Import sites.csv
#
def load_sites(path):
    """
    Load sites data from CSV file.

    :param path: Base directory path containing sites.csv
    :return: Dict of sites data
    """
    sites_path = Path(path) / "sites.csv"
    if not sites_path.exists():
        logging.error(f"Sites file {sites_path} not found")
        return {}

    try:
        with open(sites_path) as f:
            reader = csv.DictReader(f)
            sites_data = {}
            for row in reader:
                sites_data[row['basename']] = row['name']
            return sites_data
    except Exception as e:
        logging.error(f"Error loading sites.csv: {e}")
        return {}


def move_one(fn, ftype, action):
    """
    Type-independant move, dispatch to the correct one

    :param fn:
    :param ftype:
    :param action:
    :return:
    """
    if ftype == 'adsb':
        move_one_adsb(fn, action)
    else:
        move_one_drone(fn, action)


def move_one_adsb(fn, action):
    """
    Move one file into the Hve tree.

    :param fn: filename
    :param action: true does move the file
    :return: nothing
    """
    fname = Path(fn).name
    ftype = 'adsb'

    # Look for specific ADS-B filename format
    #
    fc = re.search(r'^(?P<site>.*?)([0-9]*)_(?P<year>\d+)-(?P<month>\d+)-(\d+).parquet$', fname)

    # ADS-B pattern
    #
    if fc is not None:
        site = fc.group('site')

        # Fetch the short name
        #
        try:
            site = sites[site]
        except KeyError:
            print(f'Unknown site {site}')
            return

        logging.info(f"site={site}")

        year = fc.group('year')
        month = fc.group('month')

        # Create target
        #
        ourdir = Path(datadir) / ftype / f"site={site}" / f"year={year}" / f"month={month:02}"
        if not Path(ourdir).exists():
            os.makedirs(ourdir)
        final = Path(ourdir) / fname
    else:
        print(f"Ignoring {fn}")

    logging.info(f"Moving {fn} into {final}")
    if action:
        print(f"Moving {fn} into {final}")
        Path(fn).rename(final)


def move_one_drone(fn, action):
    """
    Move one file into the Hve tree.

    :param fn: filename
    :param ftype: type of pat, drones or adsb
    :param action: true does move the file
    :return: nothing
    """
    fname = Path(fn).name
    ftype = 'drones'

    # Drone pattern
    #
    fc = re.search(r'^drones-(?P<year>\d{4})-(?P<month>\d{2})-(?P<day>\d{2}).parquet$', fname)
    if fc is not None:
        year = fc.group('year')
        month = fc.group('month')
        ourdir = Path(datadir) / ftype / f"year={year}" / f"month={month:02}"
        if not Path(ourdir).exists():
            os.makedirs(ourdir)
        final = Path(ourdir) / fname
        logging.info(f"Moving {fn} into {final}")
        if action:
            Path(fn).rename(final)
    else:
        logging.info(f"Ignoring {fn}")


# Setup arguments
#
parser = argparse.ArgumentParser(
    prog='dispatch-drops',
    description='Move each file in the right Hive directory for the given day.')

parser.add_argument('--datalake', '-D', help='Datalake is here.')
parser.add_argument('--drones', action='store_true', help='This is drone data.')
parser.add_argument('--dry-run', '-n', action='store_true', help="Do not actually move the file.")
parser.add_argument('files', nargs='*', help='List of files or directories.')
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

importdir = f"{datalake}/import"
datadir = f"{datalake}/data"
bindir = f"{datalake}/bin"
filesdir = f"{datalake}/files"
logdir = f"{datalake}/var/log"

date = datetime.now().strftime('%Y%m%d')
logfile = f"{logdir}/dispatch-drops-{date}.log"
logging.basicConfig(filemode='a', filename=logfile, level=logging.INFO, datefmt="%H:%M:%S",
                    format='%(asctime)s - %(levelname)s: %(message)s')
logging.info("Starting")

sites = load_sites(filesdir)

files = args.files
for file in files:
    # We have a directory
    #
    if os.path.isdir(file):
        print(f"Exploring {file}")
        with os.scandir(file) as base:
            for fn in base:
                if fn.name.endswith(".parquet"):
                    print(f"Looking at {fn.name}")
                    move_one(fn.path, ftype, action)
    else:
        print(f"Just {file}")
        move_one(file, ftype, action)
