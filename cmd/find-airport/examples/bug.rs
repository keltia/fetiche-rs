use polars::prelude::*;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let fname = "examples/airports.parquet";
    let name = "CDG";

    let lf = LazyFrame::scan_parquet(fname.into(), Default::default())?
        .select([
            col("ident"),
            col("name"),
            col("latitude_deg"),
            col("longitude_deg"),
            col("elevation_ft"),
            col("iata_code"),
        ])
        .filter(
            col("iata_code")
                .eq(lit(name))
                .or(col("name").str().contains(lit(name), true))
                .or(col("ident").str().contains(lit(name), true)),
        )
        .filter(col("iata_code").is_not_null())
        .collect()?;
    dbg!(&lf);
    Ok(())
}
