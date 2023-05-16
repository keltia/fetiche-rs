// Safety check
//
version = 3

site "eih" {
  type     = "drone"
  format   = "aeroscope"
  base_url = "http://127.0.0.1:2400"
  auth     = {
    login    = "SOMETHING"
    password = "NOPE"
    token    = "/login"
  }
  routes = {
    get = "/drone/get"
  }
}

site "asd" {
  type     = "drone"
  format   = "asd"
  base_url = "https://eur.airspacedrone.com/api"
  auth     = {
    login    = "USERNAME"
    password = "GUESS"
    token    = "/security/login"
  }
  routes = {
    get = "/journeys/filteredlocations/json"
  }
}

site "lux" {
  type     = "drone"
  format   = "asd"
  base_url = "https://eur.airspacedrone.com/api"
  auth     = {
    login    = "USERNAME"
    password = "GUESS"
    token    = "/security/login"
  }
  routes = {
    list = "/journeys"
    get  = "/journeys/$1"
  }
}

site "opensky" {
  type     = "adsb"
  format   = "opensky"
  base_url = "https://opensky-network.org/api"
  auth     = {
    username = "dphu"
    password = "NOPE"
  }
  cmd = {
    get = "/states/own"
  }
}

site "safesky" {
  type     = "adsb"
  format   = "safesky"
  base_url = "https://public-api.safesky.app"
  auth     = {
    api_key = "foobar"
  }
  routes = {
    get = "/v1/beacons"
  }
}
