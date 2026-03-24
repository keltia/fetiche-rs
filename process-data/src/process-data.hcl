version = 6

datalake = "/acute"

db {
  url = "http://db:8123"
  // db credentials
  user     = "roberto"
  password = "PASSWORD"
}

// List of different DB profiles: set of namespace values for different
// part of each computation.
//
profiles {
  "prod" = {
    // fetch plane data from this namespace
    "plane_db" = "acute"
    // fetch drone data from this namespace
    "drone_db" = "acute"
    // working tables will be in this namespace
    "work_db" = "acute"
  }
  "dev" = {
    "plane_db" = "acute"
    "drone_db" = "acute"
    "work_db"  = "acute_dev"
  }
}

distances {
  threshold = 1852
  factor    = 3
  place     = 70
}

