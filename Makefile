# Makefile primarily to install binaries and scripts
#
# usage:
# make DESTDIR=/acute install
#
# NOTE: This is for GNU make
#
DESTDIR ?=	/Users/acute

.VPATH = 	target/release target/debug

BINARIES =	acutectl process-data
SCRIPTS =	scripts/convert-csv.py scripts/dispatch-drops.py scripts/fetch-all-adsb.txt scripts/fetch-all-drones.py \
	scripts/fetch-asd-drones.py scripts/fetch-ftp-adsb.py scripts/fetch-opensky.py scripts/import-adsb.py \
	scripts/import-drones.py
TARGET =	target/release

all:	${BINARIES}
	cargo build --release

debug:	${BINARIES}
	cargo build

acutectl: acutectl/src/main.rs

process-data: process-data/src/main.rs

install: $(BINARIES) $(SCRIPTS)
	install -c -m 755 -s -o acute target/release/acutectl $(DESTDIR)/bin
	install -c -m 755 -s -o acute target/release/process-data $(DESTDIR)/bin
	install -c -m 755 -o acute $(SCRIPTS)  $(DESTDIR)/bin

push:
	jj git push -b develop --remote origin
	jj git push -b develop --remote gitlab
