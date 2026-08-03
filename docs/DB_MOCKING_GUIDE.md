# Database Mocking Guide for process-data Tests

## Problem

The `test_prepare_work_list` test requires database connections to ClickHouse through:
- `enumerate_sites(ctx, day)` - queries database for sites
- `find_site(ctx, name)` - queries database for a specific site

## Solutions

### Option 1: Trait-Based Dependency Injection (Recommended)

Create a trait for database operations and provide both real and mock implementations.

#### Step 1: Create a trait for database operations

```rust
// src/cmds/site_repository.rs (new file)
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use eyre::Result;
use crate::cmds::Site;

#[async_trait]
pub trait SiteRepository: Send + Sync {
    async fn find_site(&self, name: &str) -> Result<Site>;
    async fn enumerate_sites(&self, day: DateTime<Utc>) -> Result<Vec<Site>>;
}

// Real implementation
pub struct ClickHouseSiteRepository {
    pub dbh: klickhouse::bb8::Pool<klickhouse::ConnectionManager>,
}

#[async_trait]
impl SiteRepository for ClickHouseSiteRepository {
    async fn find_site(&self, name: &str) -> Result<Site> {
        // Current implementation from site.rs
        let dbh = self.dbh.get().await?;
        // ... existing query logic
    }

    async fn enumerate_sites(&self, day: DateTime<Utc>) -> Result<Vec<Site>> {
        // Current implementation from site.rs
        let dbh = self.dbh.get().await?;
        // ... existing query logic
    }
}

// Mock implementation for tests
#[cfg(test)]
pub struct MockSiteRepository {
    pub sites: Vec<Site>,
}

#[cfg(test)]
#[async_trait]
impl SiteRepository for MockSiteRepository {
    async fn find_site(&self, name: &str) -> Result<Site> {
        self.sites
            .iter()
            .find(|s| s.name == name)
            .cloned()
            .ok_or_else(|| eyre::eyre!("Site not found: {}", name))
    }

    async fn enumerate_sites(&self, _day: DateTime<Utc>) -> Result<Vec<Site>> {
        Ok(self.sites.clone())
    }
}
```

#### Step 2: Update Context to use the trait

```rust
// src/runtime.rs
pub struct Context {
    pub config: Arc<HashMap<String, String>>,
    pub dbh: Pool<ConnectionManager>,
    pub site_repo: Arc<dyn SiteRepository>,  // Add this
    pub pool_size: usize,
    pub wait: u64,
    pub dry_run: bool,
}
```

#### Step 3: Update test to use mock

```rust
#[tokio::test]
async fn test_prepare_work_list() -> Result<()> {
    // Create mock sites
    let mock_sites = vec![
        Site {
            name: "site1".to_string(),
            latitude: 48.8566,
            longitude: 2.3522,
            id: 1,
        },
        Site {
            name: "site2".to_string(),
            latitude: 51.5074,
            longitude: -0.1278,
            id: 2,
        },
        Site {
            name: "site3".to_string(),
            latitude: 40.7128,
            longitude: -74.0060,
            id: 3,
        },
    ];

    let site_repo = Arc::new(MockSiteRepository { sites: mock_sites });
    
    // Create minimal context with mock
    let ctx = Context {
        config: Arc::new(HashMap::new()),
        dbh: /* mock pool - see below */,
        site_repo,
        pool_size: 1,
        wait: 0,
        dry_run: true,
    };

    let b = "2023-10-01T00:00:00Z".parse::<Timestamp>().unwrap();
    let e = "2023-10-02T00:00:00Z".parse::<Timestamp>().unwrap();
    let dates = vec![b, e];

    let work_list = prepare_work_list(&ctx, dates, "*").await?;
    assert_eq!(work_list.len(), 6); // 2 dates × 3 sites
    Ok(())
}
```

---

### Option 2: Use `mockall` Crate (Easier, Less Invasive)

Add to `Cargo.toml`:
```toml
[dev-dependencies]
mockall = "0.13"
```

Create mockable trait without changing production code:

```rust
// In tests module
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use mockall::*;

    #[automock]
    trait SiteOperations {
        async fn find_site(&self, ctx: &Context, name: &str) -> Result<Site>;
        async fn enumerate_sites(&self, ctx: &Context, day: DateTime<Utc>) -> Result<Vec<Site>>;
    }

    #[tokio::test]
    async fn test_prepare_work_list_with_mockall() -> Result<()> {
        let mut mock = MockSiteOperations::new();
        
        // Setup expectations
        mock.expect_enumerate_sites()
            .times(2) // Called for 2 dates
            .returning(|_ctx, _day| {
                Ok(vec![
                    Site { name: "site1".into(), latitude: 48.8566, longitude: 2.3522, id: 1 },
                    Site { name: "site2".into(), latitude: 51.5074, longitude: -0.1278, id: 2 },
                    Site { name: "site3".into(), latitude: 40.7128, longitude: -74.0060, id: 3 },
                ])
            });

        // Test with mocked functions...
    }
}
```

---

### Option 3: Test Database (Most Realistic)

Use a test ClickHouse instance with docker:

```rust
// Cargo.toml dev-dependencies
[dev-dependencies]
testcontainers = "0.23"

#[tokio::test]
#[ignore] // Skip in normal test runs, requires Docker
async fn test_prepare_work_list_with_real_db() -> Result<()> {
    use testcontainers::{clients, images};
    
    let docker = clients::Cli::default();
    let clickhouse = docker.run(images::clickhouse::ClickHouse::default());
    let port = clickhouse.get_host_port_ipv4(9000);
    
    // Setup test database with test data
    let client_opts = ClientOptions::default()
        .with_url(format!("tcp://localhost:{}", port));
    
    let dbh = klickhouse::Client::connect(client_opts).await?;
    
    // Insert test data
    dbh.execute("CREATE TABLE IF NOT EXISTS sites ...").await?;
    dbh.execute("INSERT INTO sites VALUES ...").await?;
    
    // Now run the actual test
    let ctx = create_test_context(dbh).await?;
    let work_list = prepare_work_list(&ctx, dates, "*").await?;
    assert_eq!(work_list.len(), 6);
    Ok(())
}
```

---

### Option 4: Conditional Compilation (Quick Fix)

Make the test conditional on having a real database:

```rust
#[tokio::test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
async fn test_prepare_work_list() -> Result<()> {
    // Only runs when: cargo test --features integration-tests
    // ... existing test code ...
}

// Or check for environment variable
#[tokio::test]
async fn test_prepare_work_list() -> Result<()> {
    if std::env::var("TEST_DATABASE_URL").is_err() {
        eprintln!("Skipping test - TEST_DATABASE_URL not set");
        return Ok(());
    }
    // ... rest of test ...
}
```

---

### Option 5: Refactor to Accept Functions (Simplest)

Make the functions accept the database operations as parameters:

```rust
async fn prepare_work_list<F, G, Fut1, Fut2>(
    ctx: &Context,
    dates: Vec<Timestamp>,
    site_filter: &str,
    find_site_fn: F,
    enumerate_sites_fn: G,
) -> Result<Vec<WorkItem>>
where
    F: Fn(&Context, &str) -> Fut1 + Send + Sync,
    G: Fn(&Context, DateTime<Utc>) -> Fut2 + Send + Sync,
    Fut1: Future<Output = Result<Site>> + Send,
    Fut2: Future<Output = Result<Vec<Site>>> + Send,
{
    // Use find_site_fn and enumerate_sites_fn instead of direct calls
}

#[tokio::test]
async fn test_prepare_work_list() -> Result<()> {
    // Mock functions
    let mock_find_site = |_ctx: &Context, name: &str| async move {
        Ok(Site {
            name: name.to_string(),
            latitude: 48.8566,
            longitude: 2.3522,
            id: 1,
        })
    };

    let mock_enumerate_sites = |_ctx: &Context, _day: DateTime<Utc>| async move {
        Ok(vec![
            Site { name: "site1".into(), latitude: 48.8566, longitude: 2.3522, id: 1 },
            Site { name: "site2".into(), latitude: 51.5074, longitude: -0.1278, id: 2 },
            Site { name: "site3".into(), latitude: 40.7128, longitude: -74.0060, id: 3 },
        ])
    };

    let work_list = prepare_work_list(
        &ctx,
        dates,
        "*",
        mock_find_site,
        mock_enumerate_sites,
    ).await?;
    
    assert_eq!(work_list.len(), 6);
    Ok(())
}
```

---

## Recommendation

For your codebase, I recommend **Option 1 (Trait-Based DI)** or **Option 2 (mockall)** because:

1. ✅ **Clean separation** - Production code unchanged (Option 2) or minimally changed (Option 1)
2. ✅ **Fast tests** - No database needed
3. ✅ **Deterministic** - Tests won't fail due to database state
4. ✅ **CI-friendly** - No Docker/external services required
5. ✅ **Easy to extend** - Can mock other database operations later

**Option 2 (mockall)** is quickest to implement if you want minimal changes to production code.

**Option 1** is better long-term for testability and if you plan to support multiple backends.

Would you like me to implement one of these approaches?
