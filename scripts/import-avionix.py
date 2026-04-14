#! /usr/bin/env python3
#
"""
This is for importing ADS-B data into Clickhouse.

This utility takes a filename or a directory.  If the former, import the given file and if the latter
all parquets files in the tree.

XXX this is specific to the macOS version of the client, invoked as `clickhouse client` and not
`clickhouse-client` or `clickhouse-local` like in the other versions.

XXX You must have `bdt(1)` and `qsvlite(1)` somewhere in the `PATH`
"""

import argparse
import logging
import os
import sys
import tempfile
import time
from datetime import datetime
from pathlib import Path
from subprocess import run

# CONFIG CHANGE HERE or use -D
#
datalake = "/acute"
db = 'acute'
chunk = 500_000
convert_cmd = 'bdt'
delete = False
clickhouse = 'clickhouse-client'
if sys.platform.startswith('darwin'):
    clickhouse = 'clickhouse client'

# Import DB data from env.
#
host = os.getenv('CLICKHOUSE_HOST')
user = os.getenv('CLICKHOUSE_USER')
pwd = os.getenv('CLICKHOUSE_PASSWD')
dbn = os.getenv('CLICKHOUSE_DB') or db


def process_one(dir_path, fname, action):
    """
    If given a parquet file, convert it into csv and import it.

    :param dir_path directory part of the file path.
    :param fname: filename.
    :param action: do we do something or just print?
    :return: converted filename.
    """
    print(f"file={fname}")

    ext = Path(fname).suffix

    # .csv.gz ?
    #
    if ext == '.gz':
        cmd = f"gunzip {fname}"
        fname = Path(fname).stem
        ext = Path(fname).suffix
        logging.info(f"{cmd} -> {fname}")
        if action:
            ret = run(cmd, shell=True, capture_output=True)
            if ret.returncode != 0:
                logging.error("error: ", ret.stderr)
                print("error: ", ret.stderr, file=sys.stderr)
                return fname
        else:
            print(f"cmd={cmd} -> {fname}")

    if ext == '.parquet':
        logging.info(f"found parquet file {fname}")
        csv = Path(fname).with_suffix('.csv')
        if os.path.exists(os.path.join(dir_path, csv)):
            logging.warning(f"Warning: both parquet & csv exist for {fname}, ignoring parquet.")
            fname = csv
        else:
            full = os.path.join(dir_path, fname)
            new = tempfile.NamedTemporaryFile(suffix='.csv').name
            cmd = f"{convert_cmd} convert -s {full} {new}"
            logging.info(f"{cmd}")
            if action:
                ret = run(cmd, shell=True, capture_output=True)
                if ret.returncode != 0:
                    logging.error("error", "(", fname, "): ", ret.stderr)
                    print("error: ", ret.stderr, file=sys.stderr)
                    return fname
            else:
                print(f"Running {cmd}")
            fname = new

    # Split file into chunks
    #
    tmpdir = Path(fname).stem

    logging.info(f"Creating {tmpdir} and splitting {fname} into it")
    cmd = f"qsvlite split -s {chunk} {tmpdir} {fname}"
    ret = run(cmd, shell=True, capture_output=True)
    if ret.returncode != 0:
        logging.error("error", "(", fname, "): ", ret.stderr)
        print("error: ", ret.stderr, file=sys.stderr)
        return fname

    # All files in tmpdir are CSV split from main
    #
    # Import data in chunk.
    #
    for root, dirs, files in os.walk(tmpdir, topdown=True):
        logging.info(f"into {root} for split files")

        # Now do stuff, look at parquet/csv only
        #
        for f in files:
            if Path(f).suffix != '.csv':
                logging.warning(f"{f} ignored.")
                continue
            logging.info(f"Processing {f} from {root}")
            import_one_chunk(root, f)
            time.sleep(2)

    logging.info(f"insert from {tmpdir} done.")

    # Cleanup
    #
    cmd = f"/bin/rm -rf {tmpdir}"
    ret = run(cmd, shell=True, capture_output=True)
    if ret.returncode != 0:
        logging.error("error", "(", fname, "): ", ret.stderr)
        print("error: ", ret.stderr, file=sys.stderr)
        return fname
    logging.info(f"Removing {tmpdir}.")

    # Remove file if requested
    #
    if delete:
        logging.info("delete done.")
        os.remove(fname)

    return fname


def import_one_chunk(dir_path, fname):
    """
    Import one chunk of at most "chunk" lines into CH.

    :param dir_path:
    :param fname:
    :return:
    """
    logging.info(f"Processing {fname}")

    ch_cmd = f"{clickhouse} -h {host} -u {user} -d {dbn} --password {pwd} -q \"INSERT INTO avionix_raw FORMAT CsvWithNames\""
    cmd = f"/bin/cat {os.path.join(dir_path, fname)} | {ch_cmd}"
    logging.info(f"cmd={cmd}")
    if action:
        ret = run(cmd, shell=True, capture_output=True)
        if ret.returncode != 0:
            logging.error("error", "(", os.path.join(dir_path, fname), "): ", ret.stderr)
            print("error: ", ret.stderr, file=sys.stderr)
            return fname
    else:
        print(f"Running {cmd}")
    logging.info("insert done.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        prog='import-avionix',
        description='Import Avionix data into CH.')

    parser.add_argument('--chunk-size', '-S', type=int, help='Import by batch of that many lines.')
    parser.add_argument('--datalake', '-D', help='Datalake is here.')
    parser.add_argument('--dry-run', '-n', action='store_true', help="Just show what would happen.")
    parser.add_argument('--delete', '-d', action='store_true', help="Delete final file.")
    parser.add_argument('--interval', '-i', type=int, help='Interval between imports.')
    parser.add_argument('--no-delay', '-N', action='store_true', help='Do not add delay between imports.')
    parser.add_argument('files', nargs='*', help='List of files or directories.')
    args = parser.parse_args()

    if args.datalake is not None:
        datalake = args.datalake

    importdir = f"{datalake}/import"
    datadir = f"{datalake}/data/adsb"
    bindir = f"{datalake}/bin"
    logdir = f"{datalake}/var/log"
    filesdir = f"{datalake}/files"

    date = datetime.now().strftime('%Y%m%d')
    logfile = f"{logdir}/import-vionix-{date}.log"
    logging.basicConfig(filemode='a', filename=logfile, level=logging.INFO, datefmt="%H:%M:%S",
                        format='%(asctime)s - %(levelname)s: %(message)s')
    logging.info("Starting")

    action = not args.dry_run
    delete = args.delete

    if args.chunk_size is not None:
        chunk = args.chunk_size
        logging.info(f"Chunk size is {chunk} lines.")

    # Default interval between imports is 5s
    #
    if args.interval is None:
        interval = 5
    else:
        interval = args.interval
    
    if args.no_delay is False:
        logging.info(f"Delay is {interval}s")

    files = args.files
    for file in files:
        # We have a directory
        #
        if os.path.isdir(file):
            print(f"Exploring {file}")
            logging.info(f"Inside {file}")
            for root, dirs, files in os.walk(file, topdown=True):
                logging.info(f"into {root}")

                # Now do stuff, look at parquet/csv only
                #
                for f in files:
                    if Path(f).suffix != '.parquet' and Path(f).suffix != '.csv':
                        logging.warning(f"{f} ignored.")
                        continue

                    # Ignore non drones-related files
                    #
                    name = Path(f).stem
                    if not name.startswith('avionix-'):
                        logging.warning(f"{f} ignored.")
                        continue

                    r = process_one(root, f, action)
                    if r is None:
                        logging.warning(f"{f} skipped.")

                    if args.no_delay is None:
                        time.sleep(interval)
        else:
            logging.info(f"file={file}")
            root = Path(file).root
            r = process_one(root, file, action)
            if r is None:
                logging.warning(f"{file} skipped.")
