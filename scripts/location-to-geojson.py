#! /usr/bin/env python3
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

with open('locations.hcl', 'r') as file:
    list = hcl.load(file)

# Now, for all of our locations, generate the bounding box and save as GeoJSON
#
all = []
for (location, coord) in list['location'].items():
    lat = coord['lat']
    lon = coord['lon']
    bb = gen_bb(lat, lon, dist)
    all.append(Feature(geometry=bb, properties={'name': location}))

res = FeatureCollection(all)
print(res)
