#! /usr/bin/env python3
#
# Fetch all drone data from ASD in one go, creating the entire Hive-based directory tree

import argparse
import logging
import os
from datetime import datetime

years = {
    2021: [-1, -1, -1, -1, -1, -1, 31, 31, 30, 31, 30, 31],
    2022: [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31],
    2023: [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31],
    2024: [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31],
}

datalake = "/Users/acute/data"


def fetch_one_year(year: int, action: bool):
    """
    Fetch one year of drone data.

    :param year: the year we want
    :return:
    """
    print(f"Processing year {year}")

    months = years[year]
    print(months)
    for ind, days in enumerate(months):
        # Skip months we do not have any data from
        #
        month = ind + 1
        basedir = f"{datalake}/drones/year={year}/month={month:02d}"
        if not os.path.exists(basedir):
            continue
        fetch_one_month(year, month)


def fetch_one_month(year_id: int, month_id: int):
    """
    Fetch one entire month of drone data.

    :param year_id: year
    :param month_id: month
    :return:
    """
    # Move ourselves in the relevant directory
    #
    basedir = f"{datalake}/drones/year={year_id}/month={month_id:02d}"
    print(f"Processing in {basedir}")
    if os.path.exists(basedir):
        os.chdir(basedir)

    monthdays = years[year_id]
    days = monthdays[month_id - 1]
    for day in range(1, days):
        print(f"Fetching day {day}")
        fetch_one_day(year_id, month_id, day)


def fetch_one_day(year_id: int, month_id: int, day_id: int):
    """
    Fetch one day of drone data for the given year and month.

    :param year_id:
    :param month_id:
    :param day_id:
    :return:
    """
    current = f"{year_id}-{month_id:02d}-{day_id:02d}"
    print(f"Processing {current}")
    cmd = f"acutectl fetch -o drones-{current}.parquet lux-me day '{current} 00:00:00 UTC'"
    print(f"Running {cmd}")


parser = argparse.ArgumentParser(
    prog='fetch-all drones',
    description='Fetch all drone data ever in a single run.')

parser.add_argument('--datalake', '-D', help='Datalake is here.')
parser.add_argument('--dry-run', '-n', action='store_true', help="Just show what would happen.")
args = parser.parse_args()

if args.datalake is not None:
    datalake = args.datalake

if args.dry_run:
    action = False
else:
    action = True

importdir = f"{datalake}/import"
datadir = f"{datalake}/data/drones"
bindir = f"{datalake}/bin"
logdir = f"{datalake}/var/log"

date = datetime.now().strftime('%Y%m%d')
logfile = f"{logdir}/fetch-all-drones-{date}.log"
logging.basicConfig(filemode='a', filename=logfile, level=logging.INFO, datefmt="%H:%M:%S",
                    format='%(asctime)s - %(levelname)s: %(message)s')
logging.info("Starting")

fetch_one_year(2024, action)

