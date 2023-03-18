default = "local"
version = 1

db "local" {
    path = "testdata/adsb.sqlite"
}

db "mysql" {
    host     = "mysql.db.local"
    user     = "root"
    url      = "mysql://foo.example.net"
    password = "nope"
    tls      = true
}
