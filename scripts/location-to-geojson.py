#! /usr/bin/env python3
#
# Result can be imported into DuckDB with:
# ```text
# D  create table locations as select * from ST_read('all.json');
# ```
# NOTE: `spatial` extension must be loaded into DuckDB prior to this.
#
import hcl
from geojson import Point, Polygon, Feature, FeatureCollection

one_deg_nm = (40_000. / 1.852) / 360.


def gen_bb(lat, lon, dist):
    """Generate a `Polygon` 50nm wide around the specified point."""
    dist = dist / one_deg_nm
    (min_lat, max_lat) = (lat - dist, lat + dist)
    (min_lon, max_lon) = (lon - dist, lon + dist)
    x0y0 = Point((min_lon, min_lat))
    x1y0 = Point((max_lon, min_lat))
    x1y1 = Point((max_lon, max_lat))
    x0y1 = Point((min_lon, max_lat))
    return Polygon([[x0y0, x1y0, x1y1, x0y1]])


# bounding box is 25nm wide
#
dist = 25.0

# Read our HCL file
#
with open('locations.hcl', 'r') as file:
    locations = hcl.load(file)

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
