version = 1

// Describe a local directory tree used to store files
//
storage "local" {
  path     = "/var/run/acute"
  rotation = "1h"
}

storage "daily" {
  path     = "/var/run/acute/data"
  rotation = "1d"
}
