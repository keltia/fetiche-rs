version = 1

// Describe an S3-compatible bucket
//
storage "garage" {
  bucket = "acute"
  region = "garage"
}

// Describe a local directory tree used to store files
//
storage "local" {
  path = "/var/run/acute"
  rotation = "1h"
}
