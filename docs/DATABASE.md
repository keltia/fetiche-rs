# Databases

At the beginning I was looking at a database that could import all the data I need, and as I was handling time-based
data (ADS-B and drone positions) my first choice was something like [InfluDB], a time-series database. I never got to
develop support for it due to changing priorities, so I moved to a different platform to be faster.

## DuckDB

[DuckDB] is an embedded [OLAP] database like [SQLite]. It allows for some fast development because you just start using
it, no need for deploying a server and complex infrastructure.

## Databend

My first choice as an online database (with a server or a set of replicated servers), as opposed as an embedded one was
[Databend] mainly because it is written in [Rust] my main development language.

My current calculations rely heavily on the database having geospace functions, at the very least geodesic distance and
whether a given point is within a circle/rectangle. And while Databend has many geospace functions, it lacks what I
need.

## Clickhouse

[Clickhouse] was mentioned on Twitter the other day by a friend of mine, so I went to explore it, including getting
registered on the Slack server. First steps were a bit difficult (mainly because I don't know the product) but with
some help from helpful people on Slack, I was able to find my way.

Clickhouse is seriously fast

##        

[Clickhouse]: https://clickhouse.com/

[Databend]: https://www.databend.com/

[DuckDB]: https://duckdb.org/

[OLAP]: https://en.wikipedia.org/wiki/Online_analytical_processing

[SQLite]: https://sqlite.org/


