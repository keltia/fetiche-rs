# Databases

## Preamble

At the beginning I was looking at a database that could import all the data I need, and as I was handling time-based
data (ADS-B and drone positions) my first choice was something like [InfluDB], a time-series database. I never got to
develop support for it due to changing priorities, so I moved to a different platform to be faster.

## OLTP vs OLAP

OLTP is the traditional way of designing relational databases (RDBMS) where everything is stored and managed in a
row-based
storage engine. This is how MariaDB/PostgreSQL manage their data. It is usually used when one's workflow requires many
reads, writes and updates to the dataset.

OLAP is a new way of storing massive amount of similar data, such as the one collected by analytics (web data, etc.) and
metrics (used for monitoring of software systems). The way data is stored is a columnar-based scheme, which has the
benefit of data locality and allow for very aggressive compression, which does save a lot of space.

The workflow for OLAP is composed of massive writes & reads, but data is seldom updated.

Considering ACUTE needs, OLAP seemed to be more suited than OLTP. ACUTE stores all ADS-B historical data for all the
sites where antennas are and were located, back to 2021. The ADS-B data is written and never updated, like the drone
data. What is modified and updated is so little in comparison that anything is lost in the speed advantages we get
through OLAP usage.

That's why neither MariaDB (which the first generation of ACUTE was using, without storing any history) nor PostgreSQL
(which I have known and used for years) were considered.

## DuckDB

[DuckDB] is an embedded [OLAP] database like [SQLite]. It allows for some fast development because you just start using
it, no need for deploying a server and complex infrastructure. This was ideal for the first prototype of Fetiche.  
Main inconvenient is that as an embedded DB, you can not share it and any process which opened the DB owns it for the
session. No concurrent access whatsoever.

## Databend

My first choice as an online database (with a server or a set of replicated servers), as opposed as an embedded one was
[Databend] mainly because it is an OLAP DB written in [Rust] ,my main development language.

My current calculations rely heavily on the database having geospace functions, at the very least geodesic distance and
whether a given point is within a circle/rectangle. And while Databend has many geospace functions, it lacks what I
need.

## Clickhouse

I was pointed out towards [Clickhouse] by a friend of mine, so I went to explore it, including getting
registered on the Slack server. First steps were a bit difficult (mainly because I don't know the product) but with
some help from helpful people on Slack, I was able to find my way.

Clickhouse is seriously fast even with our current table design (one table for all ADS-B data and a view to perform the
necessary calculations). We have now more than 10B records in a single table. Having a single table avoids encoding the
different sites and sources for ADS-B and we are where even PostgreSQL would be hard-pressed to compete.

Another point of view is that our current budget pressure, storage space is quite complicated to expand, and the nature
of a columnar-based, compressed storage system such as [Clickhouse] is very helpful in managing the space issue.

We have, as of 26/1/2025, 9.7B records for ADS-B, all stored in less than 400 GB of disk. Each daily calculation uses
between 5M and 9M ADS-B positions out of these 10B, and takes between 500 ms and 800 ms, for 6 sites.

Our workflow is very similar to the one used by analytics so OLAP as a design choice is sound.

Another point with [Clickhouse] is that it can be either on-premises (as we have right now), or as a Cloud-based one
(and Clickhouse themselves have an offer where you can store a CH instance in your cloud tenant, but still have them
manage it). It also has a native PowerBI connector with ODBC.

The on-premises version can also be replicated and distributed on several nodes for redundancy and more concurrency.

# References

[Clickhouse]: https://clickhouse.com/

[Databend]: https://www.databend.com/

[DuckDB]: https://duckdb.org/

[OLAP]: https://en.wikipedia.org/wiki/Online_analytical_processing

[OLTP]: https://en.wikipedia.org/wiki/Online_transaction_processing

[SQLite]: https://sqlite.org/


