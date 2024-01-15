#! /usr/bin/env python3
#
# Result can be imported into DuckDB with:
# ```text
# D  create table locations as select * from ST_read('all.json');
# ```
# NOTE: `spatial` extension must be loaded into DuckDB prior to this.
#
import argparse

import hcl
from geojson import Point, Polygon, Feature, FeatureCollection

one_deg_nm = (40_000. / 1.852) / 360.


def gen_bb(lat, lon, dist):
    """Generate a `Polygon` 50nm wide around the specified point."""

    # Convert into degrees
    #
    ddist = dist / one_deg_nm

    (min_lat, max_lat) = (lat - ddist, lat + ddist)
    (min_lon, max_lon) = (lon - ddist, lon + ddist)
    x0y0 = Point((min_lon, min_lat))
    x1y0 = Point((max_lon, min_lat))
    x1y1 = Point((max_lon, max_lat))
    x0y1 = Point((min_lon, max_lat))
    return Polygon([[x0y0, x1y0, x1y1, x0y1]])


# Setup arguments
#
parser = argparse.ArgumentParser(
    prog='location-to-geojson',
    description='Generate JeoJSON from an HCL file.')

parser.add_argument('file')
parser.add_argument('-d', '--distance', type=float, default=50.0)

args = parser.parse_args()

# Read our HCL file
#
with open(args.file, 'r') as file:
    locations = hcl.load(file)

# bounding box is 25nm wide or whatever was supplied
#
dist = args.distance

print("File: ", args.file)
print("Distance: ", args.distance, "nm\n")

# Now, for all of our locations, generate the bounding box and save as GeoJSON
#
glocs = []
for (location, coord) in locations['location'].items():
    lat = coord['lat']
    lon = coord['lon']
    bb = gen_bb(lat, lon, dist)
    glocs.append(Feature(geometry=bb, properties={'name': location}))

res = FeatureCollection(glocs)
print(res)
