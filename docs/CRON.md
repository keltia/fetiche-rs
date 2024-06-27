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

This is the current crontab running on `acute.eurocontrol.fr`  on my account.

```cronexp
# m h  dom mon dow   command
# fetch drones
05      0       *       *       *       cd /acute/import && /acute/bin/fetch-asd-drones.py -D /acute -S lux-me
10      0       *       *       *       cd /acute/import && direnv exec . /acute/bin/import-drones.py -D /acute .
15      0       *       *       *       cd /acute/import && /acute/bin/dispatch-drops.py --drones -D /acute .
# Fetch ADS-B
15      8       *       *       *       cd /acute/import && /acute/bin/fetch-ftp-adsb.py -D /acute
20      8       *       *       *       cd /acute/import && /acute/bin/convert-csv.py .
22      8       *       *       *       cd /acute/import && /acute/bin/dispatch-drops.py -D /acute .
26      0       *       *       *       cd /acute/import && direnv exec . /acute/bin/import-adsb.py -D /acute .
# sync new data to NAS
0       9       *       *       *       cd /acute && rsync -avP ./ /mnt/nas/AcuteLake/
```

This is used to avoid hardcoding the different DB parameters into every script

[direnv]: https://direnv.net/


