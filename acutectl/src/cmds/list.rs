use anyhow::Result;
use tabled::{
    builder::Builder,
    settings::{object::Rows, Alignment, Modify, Style},
};

use fetiche_formats::Format;
use fetiche_sources::{Auth, Sources};

/// Fetch the list of supported formats and their description.
///
pub fn list_formats() -> Result<String> {
    let str = Format::list()?;
    Ok(str)
}

/// Fetch all the different sources available.
///
/// TODO: we need a Sites::list() like for formats-specs above.
///
pub fn list_sources(cfg: &Sources) -> Result<String> {
    let header = vec!["Name", "Type", "Format", "URL", "Auth"];

    let mut builder = Builder::default();
    builder.set_header(header);

    cfg.iter().for_each(|(n, s)| {
        let mut row = vec![];

        let dtype = s.dtype.clone().to_string();
        let format = s.format.clone();
        let base_url = s.base_url.clone();
        row.push(n);
        row.push(&dtype);
        row.push(&format);
        row.push(&base_url);
        let auth = if let Some(auth) = &s.auth {
            match auth {
                Auth::Login { .. } => "login",
                Auth::Token { .. } => "token",
                Auth::Anon => "open",
                Auth::Key { .. } => "API key",
            }
            .to_string()
        } else {
            "anon".to_owned()
        };
        let auth = &auth.clone().to_string();
        row.push(auth);
        builder.push_record(row);
    });

    let table = builder.build().with(Style::rounded()).to_string();
    Ok(table)
}
