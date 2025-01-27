// Safety check
//
version = 4

// Soon to disappear
//
site "eih" {
  features = ["fetch"]
  type     = "drone"
  format   = "aeroscope"
  base_url = "http://127.0.0.1:2400"
  routes = {
    get = "/drone/get"
  }
}

// CDG antenna is now in LUX, use my account
//
site "lux-me" {
  features = ["fetch"]
  type     = "drone"
  format   = "asd"
  base_url = "https://eur.airspacedrone.com"
  auth = {
    login    = "MINE"
    password = "NOP"
    token    = "/api/security/login"
  }
  routes = {
    get      = "/api/journeys/filteredlocations"
    journeys = "/api/journeys"
    vector   = "/api/journeys/$1"
  }
}

// CDG antenna is now in LUX
//
site "lux" {
  features = ["fetch"]
  type     = "drone"
  format   = "asd"
  base_url = "https://eur.airspacedrone.com"
  routes = {
    get      = "/api/journeys/filteredlocations"
    journeys = "/api/journeys"
    journey  = "/api/journeys/$1"
  }
}

site "opensky" {
  features = ["fetch", "stream"]
  type     = "adsb"
  format   = "opensky"
  base_url = "https://opensky-network.org/api"
  auth = {
    username = "GUESS"
    password = "NEVER"
  }
  routes = {
    stream = "/states/own"
  }
}

site "fa-belfast" {
  features = ["fetch"]
  type     = "adsb"
  format   = "flightaware"
  base_url = "firehose.flightaware.com:1501"
  auth = {
    username = ""
    password = ""
  }
  routes = {
    get    = "range"
    stream = "live"
  }
}

site "safesky" {
  features = ["fetch"]
  type     = "adsb"
  format   = "safesky"
  base_url = "https://public-api.safesky.app"
  routes = {
    get = "/v1/beacons"
  }
}

// Avionix Cube on the EIH roof - ADS-B flow
//
site "avionix-adsb" {
  features = ["stream"]
  type     = "adsb"
  format   = "cubedata"
  base_url = "tcp.aero-network.com:50007"
  auth = {
    user_key = "MAYBE"
    api_key  = "PERHAPS"
  }
  routes = {
    get = "A"
  }
}

// Avionix Cube on the EIH roof - -RemoteID flow
//
site "avionix-rid" {
  features = ["stream"]
  type     = "drone"
  format   = "cubedata"
  base_url = "tcp.aero-network.com:50007"
  auth = {
    user_key = "ORNOT"
    api_key  = "NEVER"
  }
  routes = {
    get = "RID"
  }
}

// Thales Senhive antenna on the EIH roof.
//
site "eih-senhive" {
  features = ["stream"]
  type     = "drone"
  format   = "senhive"
  base_url = "senegress.senair.io:5672"
  auth = {
    vhost    = "eurocontrol"
    username = "NOONE"
    password = "DONTTRY"
  }
}
