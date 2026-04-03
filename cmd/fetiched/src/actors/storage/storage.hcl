version = 1

basedir = "/var/run/acute"

// Describe a local directory tree used to store files
//
storage "hourly" {
  path     = ":basedir/hourly"
  rotation = "1h"
}

storage "daily" {
  path     = ":basedir/data"
  rotation = "1d"
}
