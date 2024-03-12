#! /usr/bin/env python3
#
"""
Given a list of files on the command line, uncompress each file and convert from csv into parquet

usage: convert-csv [-h] [--dry-run] [files ...]

Uncompress and convert into parquet every csv file.

positional arguments:
  files          List of files.

options:
  -h, --help     show this help message and exit
  --dry-run, -n  Do not actually move the file.

"""
import argparse
import os

from pathlib import Path


def convert_one(fn, action, delete):
    """
    Convert one file from .csv/.csv.gz into .parquet

    :param fn: filename
    :param action: true convert the file
    :param delete: true delete the csv file after conversion
    :return: nothing
    """
    fname = Path(fn).stem
    ext = Path(fn).suffix

    # We have a gzip file.
    #
    if ext == ".gz":
        print(f"Got a gzip file: {fn}")
        if action:
            os.system(f"gunzip {fn}")
        ext = Path(fname).suffix

    # Now we should have a csv, whether it has just been uncompressed or is a plain csv file should not matter
    #
    if ext == ".csv":
        outp = f"{fname}.parquet"
        print(fn, " -> ", outp)
        if action:
            os.system(f"bdt convert  -s -z {fn} {outp}")
        if delete:
            os.remove(f"{fname}{ext}")
    else:
        print(fn, "ignored")


# Setup arguments
#
parser = argparse.ArgumentParser(
    prog='convert-csv',
    description='Uncompress and convert into parquet every csv file.')

parser.add_argument('--dry-run', '-n', action='store_true', help="Do not actually move the file.")
parser.add_argument('--delete', '-d', action='store_true', help="Remove csv after conversion.")
parser.add_argument('files', nargs='*', help='List of files.')
args = parser.parse_args()

if args.dry_run:
    action = False
else:
    action = True

if args.delete:
    delete = True
else:
    delete = False

files = args.files
for file in files:
    print(f"Looking at {file}")
    convert_one(file, action, delete)
