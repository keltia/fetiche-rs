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
#
BASEDIR="/acute"
DATADIR="${BASEDIR}/data/avionix"

# Check where we are running
#
PLATFORM=$(uname)
if [ x"${PLATFORM}" = x"Darwin" ]; then
  CLIENT="clickhouse client"
else
  CLIENT="clickhouse-client"
fi

# Check arguments
#
ARG=$1
if [ x"$ARG" = x"." ]; then
  BNAME=$(date +"%Y%m%d" -d yesterday)
else
  BNAME=$ARG:r
fi

# Check file exists
#
[[ ! -f "${BNAME}.json" ]] && echo "No json" && exit 1
#
echo "Cwd is ${PWD}"
echo "Basename is ${BNAME}"
#
MONTH=$(date +%m)
YEAR=$(date +%Y)

# First step — convert into parquet
#
echo "Convert to parquet"
sort -u "${BNAME}.json" > "${BNAME}s.json" && \
  mv "${BNAME}s.json" "${BNAME}.json" &&  \
	${BASEDIR}/bin/convert-from-json -P "${BNAME}.json"

[[ ! -f "${BNAME}.parquet" ]] && echo "no parquet" && exit 1

# Second step — create our archive tree
#
echo "Create dir tree"
DESTDIR="${DATADIR}/year=${YEAR}/month=${MONTH}"
[[ ! -d "${DESTDIR}" ]] && \
	mkdir -p "${DESTDIR}"

# Third step a  — move
echo "Move files."
mv "${BNAME}.parquet" "${DESTDIR}/${BNAME}.parquet"

# Third step b  — import
#
echo "Insert into CH."
cat "${BNAME}.json" | ${CLIENT} -h $CLICKHOUSE_HOST -u $CLICKHOUSE_USER \
  --password $CLICKHOUSE_PASSWD -d $CLICKHOUSE_DB \
  -q 'INSERT INTO avionix_raw FORMAT JSONEachRow' && \
  rm -f "${BNAME}.json"
[[ $? == 0 ]] && echo "End."
