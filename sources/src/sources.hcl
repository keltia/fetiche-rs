// Safety check
//
version = 4

site "aeroscope" {
  features = ["fetch"]
  type     = "drone"
  format   = "aeroscope"
  base_url = "http://127.0.0.1:2400"
  auth     = "token"
  routes = {
    get = "/drone/get"
  }
}

site "asd" {
  features = ["fetch"]
  type     = "drone"
  format   = "asd"
  base_url = "https://eur.airspacedrone.com/api"
  auth     = "token"
  routes = {
    get = "/journeys/filteredlocations"
  }
}

site "lux" {
  features = ["fetch"]
  type     = "drone"
  format   = "asd"
  base_url = "https://eur.airspacedrone.com/api"
  auth     = "token"
  routes = {
    list = "/journeys"
    get  = "/journeys/$1"
  }
}

site "opensky" {
  features = ["fetch", "stream"]
  type     = "adsb"
  format   = "opensky"
  base_url = "https://opensky-network.org/api"
  auth     = "login"
  routes = {
    get = "/states/own"
  }
}

site "fa-belfast" {
  features = ["fetch"]
  type   = "adsb"
  format = "flightaware"
  auth = {
    login    = "USERNAME"
    password = "HIDDEN"
  }
  base_url = "firehose.flightaware.com:1501"
  routes = {
    get = "range"
    stream = "live"
  }
}

// Incomplete support.
//
site "safesky" {
  features = ["fetch"]
  type     = "adsb"
  format   = "safesky"
  base_url = "https://public-api.safesky.app"
  auth = {
    api_key = "api_key"
  }
  routes = {
    get = "/v1/beacons"
  }
}

// Avionix Cube on the roof, using the TCP Streaming server.
//
site "avionix" {
  features = ["stream"]
  type     = "drone"
  format   = "cubedata"
  base_url = "tcp.aero-network.com:50007"
  auth = {
    user_key = "USERKEY"
    api_key  = "APIKEY"
  }
}

// Thales Senhive antenna on the EIH roof.
//
// It uses AMQP
//
site "eih-senhive" {
  features = ["stream"]
  type     = "drone"
  format   = "senhive"
  base_url = "senegress.senair.io:5672"
  auth = {
    vhost    = "VHOST"
    username = "USER"
    password = "PASSWORD"
  }
}

