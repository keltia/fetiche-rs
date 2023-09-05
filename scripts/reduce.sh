#! /usr/bin/env zsh
#

for i in *full.csv; do
        j=${i:s/full/cat21/}
        cat adsb-hdr.txt $i | awk -F: '{print $8":"$7":"$25":"$31":"$4":"$5":"$3}'>$j
        xz -v $i
done
