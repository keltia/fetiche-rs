// Metadata file describing all the supported data models for `acutectl`
//
version = 2

format "aeroscope" {
  type        = "drone"
  description = "Data extracted from the DJI Aeroscope antenna."
  source      = "ASD"
  url         = "https://airspacedrone.com/"
}

format "asd" {
  type        = "drone"
  description = "Data gathered & consolidated by ASD."
  source      = "ASD"
  url         = "https://airspacedrone.com/"
}

format "opensky" {
  type        = "adsb"
  description = "Data coming from the Opensky site, mostly ADS-B."
  source      = "Opensky"
  url         = "https://opensky-network.org/"
}

format "safesky" {
  type        = "adsb"
  description = "Data coming from the Safesky site, mostly ADS-B."
  source      = "Safesky"
  url         = "https://www.safesky.app/"
}

format "cat21" {
  type        = "adsb"
  description = "Flattened ASTERIX Cat21 data for ADS-B."
  source      = "ECTL"
  url         = "https://www.eurocontrol.int/asterix/"
}

format "cat129" {
  type        = "drone"
  description = "Flattened ASTERIX Cat129 data for Drone data."
  source      = "ECTL"
  url         = "https://www.eurocontrol.int/asterix/"
}
