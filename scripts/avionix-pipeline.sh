#! /bin/zsh
#
# Short pipeline for importing/archiving Avionix data.
#
BASEDIR="/acute"
DATADIR="${BASEDIR}/data/avionix"
#
ARG=$1; shift
BNAME=$ARG:r
#
echo "Basename is ${BNAME}"
#
MONTH=$(date +%m)
YEAR=$(date +%Y)
#
sort -u "${BNAME}.json" > "${BNAME}s.json" && \
	bdt convert -s "${BNAME}s.json" "${BNAME}.csv" && \
	bdt convert -s "${BNAME}.csv" "${BNAME}.parquet"
#
[[ ! -d "${DATADIR}/year=${YEAR}/month=${MONTH}" ]] && \
	mkdir "${DATADIR}/year=${YEAR}/month=${MONTH}"
#
mv "${BNAME}.parquet" "${DATADIR}/year=${YEAR}/month=${MONTH}/"
${BASEDIR}/bin/import-avionix.py -D ${BASEDIR} -d "${BNAME}.csv" && \
	rm -f "${BNAME}s.json" "${BNAME}.json" "${BNAME}.csv" 
