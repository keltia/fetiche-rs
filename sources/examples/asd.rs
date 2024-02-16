use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use clap::{crate_name, crate_version};
use dateparser::parse;
use serde::Serialize;
use serde_json::json;
use tap::Tap;

#[derive(Debug, Serialize)]
struct Data<'a> {
    pub startTime: DateTime<Utc>,
    pub endTime: DateTime<Utc>,
    pub sources: Vec<&'a str>,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let url = "https://eur.airspacedrone.com/api/journeys/filteredlocations";
    let token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.eyJpYXQiOjE3MDgxMjEzOTIsImV4cCI6MTcwODEyNDk5Miwicm9sZXMiOlsiUk9MRV9VU0VSX0xJU1RfQURNSU4iLCJST0xFX1VTRVJfRURJVF9BRE1JTiIsIlJPTEVfR1JPVVBfTElTVF9BRE1JTiIsIlJPTEVfR1JPVVBfRURJVF9BRE1JTiIsIlJPTEVfQUlSU1BBQ0VfTElTVF9BRE1JTiIsIlJPTEVfQUlSU1BBQ0VfRURJVF9BRE1JTiIsIlJPTEVfVFJBQ0tFUl9MSVNUX0FETUlOIiwiUk9MRV9UUkFDS0VSX0VESVRfQURNSU4iLCJST0xFX0NPTU1BTkRfRURJVF9BRE1JTiIsIlJPTEVfQUxFUlRfTElTVF9BRE1JTiIsIlJPTEVfQUxFUlRfRURJVF9BRE1JTiIsIlJPTEVfVFJBRkZJQ19MSVNUIiwiUk9MRV9UUkFGRklDX0xJU1RfQURNSU4iLCJST0xFX0pPVVJORVlfTElTVCIsIlJPTEVfSk9VUk5FWV9FRElUIiwiUk9MRV9KT1VSTkVZX0xJU1RfQURNSU4iLCJST0xFX0pPVVJORVlfRURJVF9BRE1JTiIsIlJPTEVfUFJFRkxJR0hUX0xJU1QiLCJST0xFX1BSRUZMSUdIVF9MSVNUX0FETUlOIiwiUk9MRV9QUkVGTElHSFRfRURJVCIsIlJPTEVfUFJFRkxJR0hUX0VESVRfQURNSU4iLCJST0xFX1NFTkRfU01TIiwiUk9MRV9WSU9MQVRJT05fTElTVF9BRE1JTiIsIlJPTEVfRE9DVU1FTlRBVElPTl9MSVNUIiwiUk9MRV9ET0NVTUVOVEFUSU9OX0VESVRfQURNSU4iLCJST0xFX0RST05FTU9ERUxfTElTVF9BRE1JTiIsIlJPTEVfRFJPTkVNT0RFTF9FRElUX0FETUlOIiwiUk9MRV9EQVNIQk9BUkRfQURNSU4iLCJST0xFX01BQ0hJTkVfTElTVCIsIlJPTEVfTUFDSElORV9FRElUIiwiUk9MRV9QSUxPVF9MSVNUIiwiUk9MRV9QSUxPVF9FRElUIl0sInVzZXJuYW1lIjoibWFyYy5ncmF2aXNAZXVyb2NvbnRyb2wuaW50In0.b_pKieE5nVIg2CLyBnnscYyvndPR3oFB3L3tKo148YeYP9GccROe_iEKxC89jzip0oTCvKt6V60XjPrTMn7ZvJAwuytA7vNbSsAL1gbiR-zUUBdOODOGVDeAIJD3HANNeMYwfLd8cQROP1Sw0ePFJj2tYlm4fgJjX228Y_JnR2wIVOfoi96r8DdHAstZeKC4ajOFYMAWnMXfkf8BEqpOVP3MwvQC9U8W1_mON8yEe-Wrx6wCgXVO2yk0Bz9kBl1lzia-APD_za0hMkdUng0qIJ-9upA7cLyipOlS36YRZkqFTBCWaFFGmZUBJkYhAS3MJaU5P2XBbu81JRXBqPKq-ZE9ayLUnOh17Nh3sK7X3dpXSPrtyR47aFz9hUcqqTy42YVhmwLCPZU6h4aNufcM0hAd9XXidrdY8MBBCZXNkO5Mm9T5dqIIsV2i1Qe3Z68TI62h-SscRAAX7_vqpf9enYR2pb3THmlAXngXeNmxhs4Ck-F8JRIx5ffeBpJLZF6P855Gspf5LfbEojt_TZmxPmn6Yap_IB79LUCc0p0p8LdDhig6zbVTZnt2_i_qH4XVncQCUBUClnMQEGvRTnQDMpG8QY-Apxz3ww7h3JmWSV73vZc0myq1ixn21H9L0DVuY1vpnQh_-55YFAQzbDnh_Pnw98b4Va0SfLxuZGGl6qk";

    let st = parse("2024-02-15T00:00:00.000Z").unwrap();
    let et = parse("2024-02-16T00:00:00.000Z").unwrap();
    let data = Data {
        startTime: st,
        endTime: et,
        sources: vec!["as", "wi"],
    };
    dbg!(&data);

    let d_start = data.startTime.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    dbg!(&d_start);
    let d_end = data.endTime.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    dbg!(&d_end);
    let data = format!(
        "{{\"startTime\":\"{}\",\"endTime\":\"{}\",\"sources\":[\"as\",\"wi\"]}}",
        d_start, d_end
    );
    dbg!(&data);

    // r##"{"startTime": "2024-02-15T00:00:00.000Z","endTime": "2024-02-16T00:00:00.000Z","sources": ["as","wi"]}"##

    let resp = reqwest::Client::new()
        .post(url)
        .header(
            "user-agent",
            format!("{}/{}", crate_name!(), crate_version!()),
        )
        .header("content-type", "application/json")
        .bearer_auth(token)
        .body(data)
        .tap(|r| eprintln!("req={:?}", r))
        .send()
        .await?;

    let body = resp.text().await?;

    println!("resp={}", body);
    Ok(())
}
