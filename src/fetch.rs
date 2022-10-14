use crate::Config;
use anyhow::bail;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Token {
    pub access_token: String,
}

/// Fetch the access token linked to the given login/password
///
pub fn fetch_token(client: &reqwest::blocking::Client, cfg: &Config) -> String {
    // Prepare our data
    //
    let body = format!(
        "{{\"username\": \"{}\", \"password\": \"{}\"}}",
        cfg.login, cfg.password
    );

    // fetch token
    //
    let url = format!("{}/login", cfg.base_url);
    let resp = client
        .post(&cfg.base_url)
        .header("content-type", "application/json")
        .body(body)
        .send();

    let resp = match resp {
        Ok(resp) => resp.text().unwrap(),
        Err(e) => panic!("{}", e),
    };

    let res: Token = serde_json::from_str(&resp).unwrap();
    dbg!(&res);
    res.access_token.to_owned()
}

/// Using the access token obtained through `fetch_token()`, fetch the gaiven CSV data
///
pub fn fetch_csv(cfg: Config) -> String {
    // Prepare client, no need to go async
    //
    let client = reqwest::blocking::Client::new();

    // First call to gen auth token
    //
    let token = fetch_token(&client, &cfg);

    // Use the token to authenticate ourselves
    //
    let url = format!("{}/drone/get", cfg.base_url);
    let resp = client
        .get(url)
        .header("Authorization", format!("Bearer {}", token))
        .send();

    match resp {
        Ok(resp) => resp.unwrap().text().unwrap(),
        Err(e) => bail!("HTTP error: {}", e),
    }
}
