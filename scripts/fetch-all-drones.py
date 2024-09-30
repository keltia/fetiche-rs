#! /usr/bin/env python3
#
"""
usage: fetch-all-drones [-h] [--site SITE] [--datalake DATALAKE] [--year YEAR]

Fetch all drone data from ASD in one go, creating the entire Hive-based directory tree.

options:
  -h, --help            show this help message and exit
  --site SITE, -S SITE  Use this site.
  --datalake DATALAKE, -D DATALAKE
                        Datalake is here.
  --year YEAR, -Y YEAR  Fetch a specific year and not everything.
"""

import argparse
import os
from datetime import datetime

years = {
    2021: [-1, -1, -1, -1, -1, -1, 31, 31, 30, 31, 30, 31],
    2022: [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31],
    2023: [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31],
    2024: [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31],
}

# Defaults
#
datalake = "/Users/acute/data"


def fetch_one_year(year: int):
    """
    Fetch one year of drone data.

    :param year: the year we want
    :return:
    """
    print(f"Processing year {year}")

    months = years[year]
    print(months)

    today = datetime.now()
    if year > today.year:
        return

    for ind, days in enumerate(months):
        # Skip months we do not have any data from
        #
        month = ind + 1
        fetch_one_month(year, month)


def fetch_one_month(year_id: int, month_id: int):
    """
    Fetch one entire month of drone data.

    :param year_id: year
    :param month_id: month
    :return:
    """
    today = datetime.now()
    if month_id > today.month and year_id == today.year:
        return

    # Move ourselves in the relevant directory
    #
    basedir = f"{datalake}/drones/year={year_id}/month={month_id:02d}"
    if not os.path.exists(basedir):
        os.makedirs(basedir, 0o755, exist_ok=True)

    print(f"Processing in {basedir}")
    os.chdir(basedir)
    monthdays = years[year_id]
    days = monthdays[month_id - 1]
    for day in range(1, days):
        fetch_one_day(year_id, month_id, day)


def fetch_one_day(year_id: int, month_id: int, day_id: int):
    """
    Fetch one day of drone data for the given year and month.

    :param year_id:
    :param month_id:
    :param day_id:
    :return:
    """
    today = datetime.now()
    if day_id >= today.day and month_id == today.month and year_id == today.year:
        return

    print(f"Fetching day {day_id}")
    current = f"{year_id}-{month_id:02d}-{day_id:02d}"
    print(f"Processing {current}")
    cmd = f"acutectl fetch -o drones-{current}.parquet lux-me day '{current} 00:00:00 UTC'"
    print(f"Running {cmd}")
    os.system(cmd)


# Setup arguments
#
parser = argparse.ArgumentParser(
    prog='fetch-all-drones',
    description='Fetch all drone data from ASD in one go, creating the entire Hive-based directory tree.')

parser.add_argument('--site', '-S', help='Use this site.')
parser.add_argument('--datalake', '-D', help='Datalake is here.')
parser.add_argument('--year', '-Y', type=int, help='Fetch a specific year and not everything.')
args = parser.parse_args()

site = ''
if args.datalake:
    datalake = args.datalake

if args.site is not None:
    site = args.site

if args.year is not None:
    fetch_one_year(args.year)
else:
    for year in years.keys():
        fetch_one_year(year)
