#!/bin/zsh
#
# Short pipeline for importing/archiving Avionix data.
#
# If `.` is specified as filename, try to compute last day as YYYYMMDD and use it as `BNAME`
#
# FIXME This should be python (or better), not a dumb shell script
#
# NOTE CLICKHOUSE_* variables MUST be defined at run-time.

# Base config
BASEDIR="/acute"
DATADIR="${BASEDIR}/data/avionix"

# Check arguments
ARG=$1;
if [ x"$ARG" = x"." ]; then
  BNAME=$(date +"%Y%m%d" -d yesterday)
else
  BNAME=$ARG:r
fi

# Check file exists
[[ ! -f "${BNAME}.json" ]] && exit 1
#
echo "Cwd is ${PWD}"
echo "Basename is ${BNAME}"
#
MONTH=$(date +%m)
YEAR=$(date +%Y)

# First step — convert into csv & parquet
#
# FIXME columns order will be sorted, this is a datafusion BUG
#
echo "Convert to parquet"
sort -u "${BNAME}.json" > "${BNAME}s.json" && \
	bdt convert -s "${BNAME}s.json" "${BNAME}.parquet"

# Second step — create our archive tree
#
echo "Create dir tree"
[[ ! -d "${DATADIR}/year=${YEAR}/month=${MONTH}" ]] && \
	mkdir -p "${DATADIR}/year=${YEAR}/month=${MONTH}"

# Third step a  — move
echo "Move files."
mv "${BNAME}.parquet" "${DATADIR}/year=${YEAR}/month=${MONTH}/"

# Third step b  — import
#
#${BASEDIR}/bin/import-avionix.py -D ${BASEDIR} -d "${BNAME}.csv" && \
#	rm -f "${BNAME}s.json" "${BNAME}.json" "${BNAME}.csv"
echo "Insert into CH."
cat "${BNAME}s.json" | clickhouse-client -h $CLICKHOUSE_HOST -u $CLICKHOUSE_USER \
  --password $CLICKHOUSE_PASSWD -d $CLICKHOUSE_DB \
  -q 'INSERT INTO avionix_raw FORMAT JSONEachRow' && \
  rm -f "${BNAME}s.json" "${BNAME}.json"
[[ $? == 0 ]] && echo "End."
