// Safety check
//
version = 2

site "eih" {
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
  format   = "safesky"
  base_url = "https://public-api.safesky.app"
  auth     = {
    api_key = "foobar"
  }
  routes = {
    get = "/v1/beacons"
  }
}
