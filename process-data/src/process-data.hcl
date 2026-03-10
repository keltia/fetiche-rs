version = 5

datalake = "/acute"

db {
  url = "http://db:8123"
  // fetch plane data from this namespace
  plane_db = "acute"
  // fetch drone data from this namespace
  drone_db = "acute"
  // working tables will be in this namespace
  work_db = "acute_dev"
  // db credentials
  user     = "roberto"
  password = "PASSWORD"
}

distances {
  threshold = 1852
  factor    = 3
  place     = 70
}

