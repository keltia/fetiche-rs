default = "none"

sites "eih" {
  format = "aeroscope"
  base_url = "http://127.0.0.1:2400"
  auth = {
    login    = "SOMETHING"
    password = "NOPE"
    token    = "/login"
  }
  cmd = {
    get = "/drone/get"
  }
}

sites "asd" {
  format = "asd"
  base_url = "https://eur.airspacedrone.com"
  auth = {
    login    = "USERNAME"
    password = "GUESS"
    token    = "/api/security"
  }
  cmd = {
    get = "/api/journeys/filteredlocations/json"
  }
}

sites "opensky" {
  format = "opensky"
  base_url = "https://opensky-network.org/api"
  auth = {
    login = "dphu"
    password = "NOPE"
  }
  cmd = {
    get = "/state/own"
  }
}

sites "safesky" {
  format = "safesky"
  base_url = "https://public-api.safesky.app"
  auth = {
    api_key = "foobar"
  }
  cmd = {
    get = "/v1/beacons"
  }
}
