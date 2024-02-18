#! /usr/bin/env python3
#

# Outputs: CSV or Parquet
# Deps:
#   py-hcl
#
"""
This is `opensky-history` in pure Python

usage: fetch-opensky [-h] [-o OUTPUT] [-B BEGIN] [-E END] [-P] [--today] [--yesterday] file location

Fetch OpenSky historical ADS-B data.

positional arguments:
   file                  HCL file with station locations.
   location              Location like BRU or LUX.

options:
   -h, --help            show this help message and exit
   -o OUTPUT, --output OUTPUT
                         Output file.
   -B BEGIN, --begin BEGIN
                         Start of time period.
   -E END, --end END     End of time period.
   -P, --parquet         Parquet output.
   --today               Only traffic for today.
   --yesterday           Only traffic for yesterday.
"""

import argparse
from datetime import datetime, timedelta
import hcl

from pyopensky.impala import Impala

from geojson import Point, Polygon, Feature, FeatureCollection

# approximate
one_deg_nm = (40_000. / 1.852) / 360.


def gen_bb(lat, lon, dist):
    """Generate a `Polygon` 50nm wide around the specified point.

    :param lat: Latitude of station
    :param lon: Longitude of station
    :param dist: distance around station we want points
    """

    # Convert into degrees
    #
    ddist = dist / one_deg_nm

    (min_lat, max_lat) = (lat - ddist, lat + ddist)
    (min_lon, max_lon) = (lon - ddist, lon + ddist)
    return (min_lon, min_lat, max_lon, max_lat)


def check_args(args):
    """
    Check arguments time-wise.

    :param args:
    :return:
    """
    if args.today:
        begin = now
        end = now + delta
    else:
        if args.yesterday:
            begin = now - delta
            end = now
        else:
            begin = datetime.fromisoformat(args.begin)
            end = datetime.fromisoformat(args.end)
    return begin, end


# Setup arguments
#
parser = argparse.ArgumentParser(
    prog='fetch-opensky',
    description='Fetch OpenSky historical ADS-B data.')

parser.add_argument('file', help='HCL file with station locations.')
parser.add_argument('location', help='Location like BRU or LUX.')
parser.add_argument('-o', '--output', type=str, help='Output file.')
parser.add_argument('-B', '--begin', type=str, help='Start of time period.')
parser.add_argument('-E', '--end', type=str, help='End of time period.')
parser.add_argument('-P', '--parquet', action='store_true', help='Parquet output.')
parser.add_argument('--today', action='store_true', help='Only traffic for today.')
parser.add_argument('--yesterday', action='store_true', help='Only traffic for yesterday.')
args = parser.parse_args()

# What is now
#
now = datetime.utcnow()
now = datetime(now.year, now.month, now.day, 0, 0, 0)
delta = timedelta(days=1)

# Analyse arguments.
#
begin, end = check_args(args)

# Read our HCL file
#
with open(args.file, 'r') as file:
    locations = hcl.load(file)

# Retrieve coordinates of station and calculate the bounding box.
#
all = locations['location']
coords = all[args.location]
bb = gen_bb(coords['lon'], coords['lat'], dist=70)

# Connect to Impala
#
impala = Impala()

print("From: ", begin, "To: ", end, "BB=", bb)

# Send request
#
df = impala.history(start=begin, stop=end, bounds=bb)

# Write output
#
if df is None:
    print("No data received.")
else:
    # whoever designed this API is bad
    #
    print(df.count(0)[0], " rows received.")
    if args.parquet:
        df.to_parquet(args.output, compression='zstd')
    else:
        df.to_csv(args.output)
