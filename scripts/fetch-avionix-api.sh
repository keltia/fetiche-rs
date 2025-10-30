#! /bin/zsh
#
# AVIONIX_API_KEY & AVIONIX_USER_KEY MUST be defined
#
while true; do
    fn=$(date +"%Y%m%d")
    curl  "https://aero-network.com/api/json?api-key=${AVIONIX_API_KEY}&user-key=${AVIONIX_USER_KEY}" | \
      jq --compact-output '.[]' >>$fn.json &&  ls -l $fn.json
    sleep 5
done
