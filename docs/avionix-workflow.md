# Current Workflow for fetching data from Avionix

We now use the script called `fetch-avionix-api.sh`  from the `scripts`  directory. This script will fetch the data
from the Avionix API and then convert it to CSV and Parquet. The CSV and Parquet files are then moved into the
appropriate places. The script will also clean up any files that are no longer needed.

```shell
# Weed out possible duplicates
sort -u 20251026.json >20251026s.json

# Generate CSV & Parquet
bdt convert -s 20251026s.json avionix-20251026.csv
bdt convert -s avionix-20251026.csv avionix-20251026.parquet

# Move files into places
mv avionix-20251026.parquet /acute/data/avionix/year=2025/month=10
mv avionix-20251026.csv /acute/import

# Cleanup
rm -f 20251026.json 20251026s.json
fd -S 0b -x rm

# Now import all csv
/acute/bin/import-avionix.py -D /acute -d avionix-20251026.csv
```
