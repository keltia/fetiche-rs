// Metadata file describing all the supported data models for `acutectl`
//
version = 2

format "ACSV" {
  type        = "write"
  description = "Annotated CSV, created by InfluxDB."
  source      = "InfluxData"
  url         = "https://docs.influxdata.com/influxdb/cloud/reference/syntax/annotated-csv/"
}

format "Avro" {
  type        = "write"
  description = "Row-oriented tabular format from Apache Arrow."
  source      = "Apache"
  url         = "https://avro.apache.org/"
}

format "CSV" {
  type        = "write"
  description = "Comma Separated Values aka your friend CSV."
  source      = "IBM"
  url         = "https://en.wikipedia.org/wiki/CSV"
}

format "Parquet" {
  type        = "write"
  description = "Apache Parquet export for drone/ADS-B data."
  source      = "Apache"
  url         = "https://parquet.apache.org/docs/file-format/"
}
