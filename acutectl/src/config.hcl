// Safety check
//
version = 1

site "eih" {
  auth = {
    login    = "SOMETHING"
    password = "NOPE"
    token    = "/login"
  }
}

site "asd" {
  auth = {
    login    = "USERNAME"
    password = "GUESS"
    token    = "/security/login"
  }
}

site "lux" {
  auth = {
    login    = "USERNAME"
    password = "GUESS"
    token    = "/security/login"
  }
}

site "opensky" {
  auth = {
    username = "dphu"
    password = "NOPE"
  }
}

site "safesky" {
  auth = {
    api_key = "foobar"
  }
}
