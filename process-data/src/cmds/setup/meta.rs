//! Setup metatada tables in ClickHouse.
//!
//! This module is for the metadata tables: `sites`, `antennas`, `installations`
//!
use crate::cmds::DBVars;
use crate::make_query;
use crate::runtime::Context;

use eyre::Result;

#[tracing::instrument(skip(ctx))]
pub async fn add_sites_table(ctx: &Context) -> Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let r = make_query!(
        r##"
CREATE TABLE IF NOT EXISTS {workdb}.sites (
    id              INT,
    name            VARCHAR NOT NULL,
    code            VARCHAR,
    basename        VARCHAR NOT NULL,
    latitude        Float64 NOT NULL,
    longitude       Float64 NOT NULL,
    ref_altitude    INT NOT NULL,
    timezone        VARCHAR NOT NULL,
    offset          INT NOT NULL
)
ENGINE = MergeTree
PRIMARY KEY (id)
COMMENT 'All sites with an antenna in time.'
    "##,
        dbvars
    );

    Ok(dbh.execute(&r).await?)
}

#[tracing::instrument(skip(ctx))]
pub async fn drop_sites_table(ctx: &Context) -> Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let r = make_query!(r##"DROP TABLE IF EXISTS {workdb}.sites"##, dbvars);
    Ok(dbh.execute(&r).await?)
}

// -----

#[tracing::instrument(skip(ctx))]
pub async fn add_antennas_table(ctx: &Context) -> Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let r = make_query!(
        r##"
CREATE TABLE IF NOT EXISTS {workdb}.antennas (
    id          INT,
    type        VARCHAR NOT NULL,
    name        VARCHAR NOT NULL,
    owned       BOOLEAN,
    description VARCHAR
)
ENGINE = MergeTree
PRIMARY KEY (id)
COMMENT 'All known antennas.'
    "##,
        dbvars
    );
    Ok(dbh.execute(&r).await?)
}

#[tracing::instrument(skip(ctx))]
pub async fn drop_antennas_table(ctx: &Context) -> Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let r = make_query!(r##"DROP TABLE IF EXISTS {workdb}.antennas"##, dbvars);
    Ok(dbh.execute(&r).await?)
}

// -----

#[tracing::instrument(skip(ctx))]
pub async fn add_installations_table(ctx: &Context) -> Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let r = make_query!(
        r##"
CREATE TABLE IF NOT EXISTS {workdb}.installations (
    id          INT,
    site_id     INT NOT NULL,
    antenna_id  INT NOT NULL,
    start_at    DateTime('UTC') NOT NULL,
    end_at      DateTime('UTC') NOT NULL,
    comment     VARCHAR
)
ENGINE = MergeTree
PRIMARY KEY (id)
ORDER BY (id)
COMMENT 'Which antenna on which site is installed at which time.'
    "##,
        dbvars
    );

    Ok(dbh.execute(&r).await?)
}

#[tracing::instrument(skip(ctx))]
pub async fn drop_installations_table(ctx: &Context) -> Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let r = make_query!(r##"DROP TABLE IF EXISTS {workdb}.installations"##, dbvars);
    Ok(dbh.execute(&r).await?)
}
