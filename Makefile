# Makefile primarily to install binaries and scripts
#
# usage:
# make DATALAKE=/acute install
#
# NOTE: This is for GNU make
#
DESTDIR ?=	/Users/acute

.VPATH = 	target/release

BINARIES =	acuteclt process-data
SCRIPTS =	convert-csv.py dispatch-drops.py fetch-ftp-adsb.py ftp-all-adsb.txt
TARGET =	target/release

all:

acutectl:
	cd $@ && cargo build --release

process-data:
	cd $@ && cargo build --release

install: $(BINARIES) $(SCRIPTS)
	install -c -m 755 -s -o acute target/release/acutectl $(DESTDIR)/bin
	install -c -m 755 -s -o acute target/release/process-data $(DESTDIR)/bin
	install -c -m 755 -o acute scripts/*.py $(DESTDIR)/bin
	install -c -m 644 -o acute scripts/*.txt $(DESTDIR)/bin

