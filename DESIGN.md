
# General design for the converter

## Output format

We defined a CSV output format hiding as pseudo-Cat21 Asterix format with the same field names.

## Input format

The `format` crate/module is here to define the format of the input regardless of how it is fetched.  It also defines each rules/calculations are done to convert into the output format, always our own CSV-based Cat-21-like.

## Process

1. read configuration file
   1.1. there can be several sites defined in `config.toml`
   1.2. source format is defined in Config or specified on CLI
   1.3. default source format is Aeroscope
2. read command-line flags
   2.1. input is either file or (source from Config aka network)
   2.2. if input is file, format is needed
   2.3. if input is network, format is in Config
3. create task with parameters
4. execute task
   4.1. 


