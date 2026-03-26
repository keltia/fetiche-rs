//! This module provides functionality to track and query the history of data processing runs,
//! particularly focused on aircraft and drone tracking analysis. It includes:
//!
//! - Recording processing run outcomes and statistics
//! - Tracking different completion statuses (success, missing data, no encounters)
//! - Querying historical daily_stats by date or site
//! - Storing run metadata like site information and statistics
//!
//! The module interfaces with a database to persist this information for analysis
//! and auditing purposes.
//!
use chrono::{DateTime, Utc};
use eyre::Result;
use klickhouse::{Client, QueryBuilder, Row};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use crate::cmds::{find_site_by_id, DBVars};
use crate::make_query;

/// Represents the completion status of a data processing run.
///
/// This enum indicates different possible outcomes of processing aircraft and drone data,
/// including successful completion and various failure conditions related to missing data
/// or lack of relevant encounters.
///
#[allow(dead_code)]
#[derive(Debug)]
#[repr(u8)]
pub enum RecordStatus {
    /// Processing completed successfully with all expected data
    Done = 0,
    /// Day skipped due to missing or insufficient aircraft ADS-B data
    NoPlanes,
    /// Day skipped due to missing or insufficient drone tracking data
    NoDrones,
    /// Processing completed but no potential aircraft-drone encounters were identified
    NoPotential,
    /// Processing completed but no new close encounters were found beyond existing daily_stats
    NoNewEncounters,
}

/// This is the context for which we are dealing with data processing runs.
///
/// This allow to target another db, not just the default one (think test/production).
///
#[allow(dead_code)]
pub struct History {
    /// database handle.
    pub dbh: Client,
    /// DB environment
    pub dbvars: Arc<DBVars>,
}

impl Debug for History {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("History")
            .field("dbh", &String::from("Clickhouse client"))
            .field("dbvars", &self.dbvars)
            .finish()
    }
}

impl History {
    /// Creates a new History instance for tracking data processing daily_stats
    ///
    /// # Arguments
    /// * `dbh` - A reference to the database client
    /// * `dbname` - Name of the database to connect to
    ///
    /// # Returns
    /// A new History instance configured with the provided database connection
    #[allow(dead_code)]
    #[tracing::instrument(skip(dbh))]
    pub async fn new(dbh: &Client, dbname: String) -> Self {
        let dbvars = DBVars {
            workdb: dbname.clone(),
            ..Default::default()
        };
        Self { dbh: dbh.clone(), dbvars: dbvars.into() }
    }
}

/// This records the status and statistics of a `process-data distances` run.
///
#[allow(dead_code)]
#[derive(Debug, Row)]
pub struct Record {
    /// Specific day of the run.
    pub day: DateTime<Utc>,
    /// The site id.
    pub site_id: i32,
    /// The site name.
    pub site_name: String,
    /// The status of the run.
    pub status: u8,
    /// The stats of the run (JSON encoded).
    pub stats: String,
    /// Comment, if any.
    pub comment: String,
}

impl History {
    /// Inserts a new processing record into the database
    ///
    /// # Arguments
    /// * `day` - The date of the processing run
    /// * `site_id` - Identifier for the site being processed
    /// * `status` - Status outcome of the processing run
    /// * `stats` - JSON-encoded statistics from the run
    /// * `comment` - Optional comment about the run
    ///
    /// # Returns
    /// Result indicating success or failure of the insert operation
    ///
    #[allow(dead_code)]
    #[tracing::instrument(skip(self))]
    pub async fn insert(&mut self, day: DateTime<Utc>, site_id: u32, status: RecordStatus, stats: String, comment: String) -> Result<()> {
        let dbh = self.dbh.clone();
        let dbvars = self.dbvars.clone();

        let site = find_site_by_id(&dbh, site_id).await?;
        let site_name = site.name.clone();
        let status = status as u8;

        let rq = make_query!(r##"
INSERT INTO {workdb}.daily_stats (day, site_id, site_name, status, stats, comment) ($1, $2, $3, $4, $5, $6)
    "##, dbvars);

        let q = QueryBuilder::new(&rq).arg(day).arg(site_id).arg(site_name).arg(status).arg(stats).arg(comment);
        Ok(dbh.execute(q).await?)
    }

    /// Retrieves all processing daily_stats for a specific day
    ///
    /// # Arguments
    /// * `day` - The date to query daily_stats for
    ///
    /// # Returns
    /// Vector of Record instances matching the specified day
    ///
    #[allow(dead_code)]
    #[tracing::instrument(skip(self))]
    pub async fn get_by_day(&mut self, day: DateTime<Utc>) -> Result<Vec<Record>> {
        let dbh = self.dbh.clone();
        let dbvars = self.dbvars.clone();

        let qr = make_query!(r##"SELECT * FROM {workdb}.daily_stats WHERE day = $1"##, dbvars);

        let q = QueryBuilder::new(&qr).arg(day);
        let daily_stats = dbh.query_collect::<Record>(q).await?;
        Ok(daily_stats)
    }

    /// Retrieves all processing daily_stats for a specific site
    ///
    /// # Arguments
    /// * `site` - Name of the site to query daily_stats for
    ///
    /// # Returns
    /// Vector of Record instances associated with the specified site
    ///
    #[allow(dead_code)]
    #[tracing::instrument(skip(self))]
    pub async fn get_by_site(&mut self, site: &str) -> Result<Vec<Record>> {
        let dbh = self.dbh.clone();
        let dbvars = self.dbvars.clone();

        let qr = make_query!(r##"SELECT * FROM {workdb}.daily_stats WHERE site_name = $1"##, dbvars);

        let q = QueryBuilder::new(&qr).arg(site);
        let daily_stats = dbh.query_collect::<Record>(q).await?;
        Ok(daily_stats)
    }

    /// Retrieves all incomplete processing records from the database
    ///
    /// This method queries the database for all processing records that have a non-zero
    /// status, indicating incomplete or failed processing runs. This includes cases where
    /// data was missing, no encounters were found, or other failure conditions occurred.
    ///
    /// # Returns
    /// Vector of Record instances representing all incomplete processing runs across all
    /// sites and dates
    ///
    #[allow(dead_code)]
    #[tracing::instrument(skip(self))]
    pub async fn get_incomplete(&mut self) -> Result<Vec<Record>> {
        let dbh = self.dbh.clone();
        let dbvars = self.dbvars.clone();

        let qr = make_query!(r##"SELECT * FROM {workdb}.daily_stats WHERE status != 0"##, dbvars);

        let daily_stats = dbh.query_collect::<Record>(qr).await?;
        Ok(daily_stats)
    }

    /// Retrieves all incomplete processing records for a specific day
    ///
    /// This method queries the database for all processing records associated with the given day
    /// that have a non-zero status, indicating incomplete or failed processing runs.
    ///
    /// # Arguments
    /// * `day` - The date to query incomplete records for
    ///
    /// # Returns
    /// Vector of Record instances representing incomplete processing runs for the specified day
    ///
    #[allow(dead_code)]
    #[tracing::instrument(skip(self))]
    pub async fn get_incomplete_by_day(&mut self, day: DateTime<Utc>) -> Result<Vec<Record>> {
        let dbh = self.dbh.clone();
        let dbvars = self.dbvars.clone();

        let qr = make_query!(r##"SELECT * FROM {}.daily_stats WHERE status != 0 AND day = $1"##, dbvars);

        let q = QueryBuilder::new(&qr).arg(day);
        let daily_stats = dbh.query_collect::<Record>(q).await?;
        Ok(daily_stats)
    }

    /// Retrieves all incomplete processing records for a specific site
    ///
    /// This method queries the database for all processing records associated with the given site
    /// that have a non-zero status, indicating incomplete or failed processing runs.
    ///
    /// # Arguments
    /// * `site` - Name of the site to query incomplete records for
    ///
    /// # Returns
    /// Vector of Record instances representing incomplete processing runs for the specified site
    ///
    #[allow(dead_code)]
    #[tracing::instrument(skip(self))]
    pub async fn get_incomplete_by_site(&mut self, site: &str) -> Result<Vec<Record>> {
        let dbh = self.dbh.clone();
        let dbvars = self.dbvars.clone();

        let qr = make_query!(r##"SELECT * FROM {workdb}.daily_stats WHERE status != 0 AND site_name = $1"##, dbvars);

        let q = QueryBuilder::new(&qr).arg(site);
        let daily_stats = dbh.query_collect::<Record>(q).await?;
        Ok(daily_stats)
    }

    /// Retrieves all processing records where aircraft ADS-B data was missing
    ///
    /// # Returns
    /// Vector of Record instances where processing failed due to missing aircraft data
    ///
    #[allow(dead_code)]
    #[tracing::instrument(skip(self))]
    pub async fn get_missing_adsb(&mut self) -> Result<Vec<Record>> {
        let dbh = self.dbh.clone();
        let dbvars = self.dbvars.clone();

        let qr = make_query!(r##"SELECT * FROM {workdb}.daily_stats WHERE status = 1"##, dbvars);

        let daily_stats = dbh.query_collect::<Record>(&qr).await?;
        Ok(daily_stats)
    }

    /// Retrieves all processing records where drone tracking data was missing
    ///
    /// # Returns
    /// Vector of Record instances where processing failed due to missing drone data
    ///
    #[allow(dead_code)]
    #[tracing::instrument(skip(self))]
    pub async fn get_missing_drones(&mut self) -> Result<Vec<Record>> {
        let dbh = self.dbh.clone();
        let dbvars = self.dbvars.clone();

        let qr = make_query!(r##"SELECT * FROM {workdb}.daily_stats WHERE status = 2"##, dbvars);

        let daily_stats = dbh.query_collect::<Record>(&qr).await?;
        Ok(daily_stats)
    }
}
