# cron jobs

## acute VM

### `direnv`

[direnv] a special utility that enable users to have a set of environment variables for specific directories,
both in interactive and unattended mode (i.e. cron jobs). You create a `.envrc` file there, use `direnv allow .`  to
enable direnv usage and every time you enter this directory, variables will be read and defined. As soon as you leave
said directory, all these are removed.

This needs a shell-specific hook of course. For `zsh` it goes line this inside your `.zshrc`:

```shell
eval "$(direnv hook zsh)"
```

For cron jobs, we use the `direnv exec` command wrapper.

### `crontab -l`

This is the current crontab running on `acute.eurocontrol.fr`  on my account. These are UTC times.

```cronexp
# fetch drones
05      0       *       *       *       cd /acute/import && /acute/bin/fetch-asd-drones.py -D /acute -S lux-me
10      0       *       *       *       cd /acute/import && direnv exec . /acute/bin/import-drones.py -D /acute .
15      0       *       *       *       cd /acute/import && /acute/bin/dispatch-drops.py --drones -D /acute .
# Fetch ADS-B
15      5       *       *       *       cd /acute/import && /acute/bin/fetch-ftp-adsb.py -D /acute
20      5       *       *       *       cd /acute/import && /acute/bin/convert-csv.py .
25      5       *       *       *       cd /acute/import && /acute/bin/dispatch-drops.py -D /acute .
27      5       *       *       *       cd /acute/import && direnv exec . /acute/bin/import-adsb.py -D /acute -d .
# Calculations
0       6       *       *       *       cd /acute/import && direnv exec . /acute/bin/process-data -F /acute/var/log distances planes ALL yesterday
10      6       *       *       *       cd /acute/import && /acute/bin/export-encounters.py -D /acute -d /acute/encounters
12      6       *       *       *       cd /acute/import && /acute/bin/export-encounters.py -D /acute -d /acute/encounters -S
# sync new data to NAS
0       7       *       *       *       cd /acute && rsync -avP -@1 ./ /mnt/nas/AcuteLake/
```

This is used to avoid hardcoding the different DB parameters into every script

This is the crontab running on `mac-studio`  to synchronise data between the CNSLab and inside resources including the
shared drive `AcuteLake`.

These are local time (Europe/Paris).

```cronexp
0   8    *      *       *       cd /Users/acute && rsync -avP acute-sync:/acute/{encounters,data,files} .
# windows shares need this apparently
10  8    *      *       *   cd /Users/acute && rsync -avP -@1 {encounters,data,files} /Volumes/Corporate/AcuteLake/
```

### `.envrc`

The current `.envrc` is as follows, suitable for all POSIX/Bourne shell variants:

```shell
export CLICKHOUSE_URL=http://reku.eurocontrol.fr:8123
export KLICKHOUSE_URL=reku.eurocontrol.fr:9000
export CLICKHOUSE_HOST=reku.eurocontrol.fr
export CLICKHOUSE_DB=acute
export CLICKHOUSE_USER=default
export CLICKHOUSE_PASSWD=**replace**
```

[direnv]: https://direnv.net/


