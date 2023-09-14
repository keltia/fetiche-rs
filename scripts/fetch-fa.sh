#! /bin/zsh
#
## Fetch all Flightaware data for the specific dates:
#
# 23-31/5/2023
# 6/2023
# 7/2023
# 8/2023
#

CMD="acutectl fetch"
CONV="--into cat21"
FMT="-B \"%s\" -E \"%s\""
DFMT1="2023-%s-%s 00:00:00 UTC"
DFMT2="2023-%s-%s 23:59:59 UTC"
OUTPUT="-o %s-full.csv"
SITE="fa-belfast"

function generate_cmd() {
	day=$1
	month=$2

	DAY1=$(printf -- "$DFMT1" $month $day)
	DAY2=$(printf -- "$DFMT2" $month $day)
	STR=$(printf -- "$FMT" "$DAY1" "$DAY2")
	fn=$(printf -- "2023%s%s" $month $day)
	FNAME=$(printf -- "$OUTPUT" "$fn")
	echo "$CMD $STR $CONV $FNAME $SITE"
}

function one_month() {
  num=$1
  shift

  printf "Month %d\n" $num
	for day in $(echo $*)
	do
		ACUTECTL=$(generate_cmd $day $num)
		echo "$ACUTECTL"
		eval $ACUTECTL
	done
}

# May
#
may="23 24 25 26 27 28 29 30 31"

# June/July/August
#
#june=$(jot -w "%02d" 30)
#july=$(jot -w "%02d" 31)
#aug=$(jot -w "%02d" 31)
sept=$(jot -w "%02d" 13)

# Run for each month
#
#one_month 05 $may
#one_month 06 $june
#one_month 07 $july
#one_month 08 $aug
one_month 09 $sept

echo "---"


