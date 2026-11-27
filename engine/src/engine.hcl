version = 3

basedir = "/var/db/acute"

# Optional daemon controls. If omitted, workers follows available parallelism.
workers = 2
tick    = { secs = 2, nanos = 0 }
sync    = { secs = 30, nanos = 0 }

pipeline_queue_capacity = 16
read_max_bytes          = 67108864

# Use absolute paths for local storage.
storage = {
  hourly = {
    path     = "/var/db/acute/hourly"
    rotation = "1h"
  }
  hive = {
    path = "/var/db/acute/hive"
  }
  cache = {
    url = "redis://127.0.0.1:6379"
  }
}
