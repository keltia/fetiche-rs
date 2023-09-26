
version = 1

db "local" {
  type        = "influxdb"
  url         = ""
  description = "Influx Time-series DB local instance."
}

db "drone2" {
  type        = "mysql"
  url         = ""
  description = "MariaDB for acute."
}
