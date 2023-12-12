version = 1

db "local" {
  type = "InfluxDB"
  url         = ""
  description = "Influx Time-series DB local instance."
}

db "drone2" {
  type = "Mysql"
  url         = ""
  description = "MariaDB for acute."
}
