#! /bin/zsh
#
# Short pipeline for importing/archiving Avionix data.
#
# If `.` is specified as filename, try to compute last day as YYYYMMDD and use it as `BNAME`
#
# FIXME This should be python (or better), not a dumb shell script

# Base config
BASEDIR="/acute"
DATADIR="${BASEDIR}/data/avionix"

# Check arguments
ARG=$1; shift
if [ x"$ARG" = x"." ]; then
  BNAME=$(date +"%Y%m%d" -d yesterday)
else
  BNAME=$ARG:r
fi

# Check file exists
[[ ! -f "${BNAME}.json"]] && exit 1
#
echo "Basename is ${BNAME}"
#
MONTH=$(date +%m)
YEAR=$(date +%Y)

# First step — convert into csv & parquet
sort -u "${BNAME}.json" > "${BNAME}s.json" && \
	bdt convert -s "${BNAME}s.json" "${BNAME}.csv" && \
	bdt convert -s "${BNAME}.csv" "${BNAME}.parquet"

# Second step — create our archive tree
[[ ! -d "${DATADIR}/year=${YEAR}/month=${MONTH}" ]] && \
	mkdir -p "${DATADIR}/year=${YEAR}/month=${MONTH}"

# Third step a  — move
mv "${BNAME}.parquet" "${DATADIR}/year=${YEAR}/month=${MONTH}/"

# Third step b  — import
${BASEDIR}/bin/import-avionix.py -D ${BASEDIR} -d "${BNAME}.csv" && \
	rm -f "${BNAME}s.json" "${BNAME}.json" "${BNAME}.csv" 
