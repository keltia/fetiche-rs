// Safety check
//
version = 4

site "aeroscope" {
  features = ["fetch"]
  type     = "drone"
  format   = "aeroscope"
  base_url = "http://127.0.0.1:2400"
  auth     = "token"
  routes   = {
    get = "/drone/get"
  }
}

site "asd" {
  features = ["fetch"]
  type     = "drone"
  format   = "asd"
  base_url = "https://eur.airspacedrone.com/api"
  auth     = "token"
  routes   = {
    get = "/journeys/filteredlocations/json"
  }
}

site "lux" {
  features = ["fetch"]
  type     = "drone"
  format   = "asd"
  base_url = "https://eur.airspacedrone.com/api"
  auth     = "token"
  routes   = {
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
  cmd      = {
    get = "/states/own"
  }
}

site "safesky" {
  features = ["fetch"]
  type     = "adsb"
  format   = "safesky"
  base_url = "https://public-api.safesky.app"
  auth     = "api_key"
  routes   = {
    get = "/v1/beacons"
  }
}
