default = "local"

sites "local" {
  name = "local"
  path = "testdata/adsb.sqlite"
}

sites "mysql" {
  host     = "mysql.db.local"
  user     = "something"
  password = "nope"
  tls      = true
}
