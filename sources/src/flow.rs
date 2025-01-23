use crate::{AsyncStreamable, Fetchable, Streamable};
use fetiche_formats::Format;

/// Represents the mode of data flow for different sources.
///
/// The `Flow` enum acts as a wrapper around three different traits:
/// - [`Fetchable`]: For fetching data through APIs or other methods.
/// - [`Streamable`]: For streaming data in a synchronous manner.
/// - [`AsyncStreamable`]: For streaming data asynchronously.
///
/// The primary purpose of this type is to provide a unified interface to interact
/// with the underlying objects using their specific traits, by enabling dynamic dispatch.
///
/// # Variants
///
/// - **Fetchable**
///   Wraps an object implementing the [`Fetchable`] trait.
///
/// - **Streamable**
///   Wraps an object implementing the [`Streamable`] trait.
///
/// - **AsyncStreamable**
///   Wraps an object implementing the [`AsyncStreamable`] trait.
///
/// # Examples
///
/// ```rust
/// use std::sync::mpsc::channel;
/// use fetiche_sources::{Fetchable, Streamable, AsyncStreamable, Flow};
/// use fetiche_formats::Format;
/// use eyre::Result;
///
/// #[derive(Debug)]
/// struct MockFetcher;
///
/// impl Fetchable for MockFetcher {
///     fn name(&self) -> String {
///         "MockFetcher".to_string()
///     }
///     fn authenticate(&self) -> Result<String, fetiche_sources::AuthError> {
///         Ok("test_token".to_string())
///     }
///     fn fetch(&self, out: std::sync::mpsc::Sender<String>, token: &str, args: &str) -> Result<()> {
///         out.send(format!("Data fetched with token: {token} and args: {args}"))?;
///         Ok(())
///     }
///     fn format(&self) -> Format {
///         Format::Asd
///     }
/// }
///
/// let fetcher = Flow::Fetchable(Box::new(MockFetcher {}));
/// println!("Name: {}", fetcher.name());
/// println!("Format: {:?}", fetcher.format());
/// ```
///
#[derive(Debug)]
pub enum Flow {
    Fetchable(Box<dyn Fetchable>),
    Streamable(Box<dyn Streamable>),
    AsyncStreamable(Box<dyn AsyncStreamable>),
}

impl Flow {
    /// Return the name of the underlying object
    ///
    #[inline]
    pub fn name(&self) -> String {
        match self {
            Flow::Fetchable(s) => s.name(),
            Flow::Streamable(s) => s.name(),
            Flow::AsyncStreamable(s) => s.name(),
        }
    }

    /// Return the format of the underlying object
    ///
    #[inline]
    pub fn format(&self) -> Format {
        match self {
            Flow::Fetchable(s) => s.format(),
            Flow::Streamable(s) => s.format(),
            Flow::AsyncStreamable(s) => s.format(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    #[derive(Debug)]
    struct MockFetcher;

    impl Fetchable for MockFetcher {
        fn name(&self) -> String {
            "MockFetcher".to_string()
        }

        fn authenticate(&self) -> Result<String, AuthError> {
            Ok("test_token".to_string())
        }

        fn fetch(&self, _out: Sender<String>, _token: &str, _args: &str) -> Result<()> {
            Ok(())
        }

        fn format(&self) -> Format {
            Format::Asd
        }
    }

    #[derive(Debug)]
    struct MockStreamable;

    impl Streamable for MockStreamable {
        fn name(&self) -> String {
            "MockStreamable".to_string()
        }

        fn authenticate(&self) -> Result<String, AuthError> {
            Ok("stream_token".to_string())
        }

        fn stream(&self, _out: Sender<String>, _token: &str, _args: &str) -> Result<()> {
            Ok(())
        }

        fn format(&self) -> Format {
            Format::Csv
        }
    }

    #[derive(Debug)]
    struct MockAsyncStreamable;

    #[async_trait]
    impl AsyncStreamable for MockAsyncStreamable {
        fn name(&self) -> String {
            "MockAsyncStreamable".to_string()
        }

        async fn authenticate(&self) -> Result<String, AuthError> {
            Ok("async_token".to_string())
        }

        async fn stream(&self, _out: Sender<String>, _token: &str, _args: &str) -> Result<()> {
            Ok(())
        }

        fn format(&self) -> Format {
            Format::Json
        }
    }

    #[test]
    fn test_flow_name() {
        let fetchable_flow = Flow::Fetchable(Box::new(MockFetcher {}));
        let streamable_flow = Flow::Streamable(Box::new(MockStreamable {}));
        let async_streamable_flow = Flow::AsyncStreamable(Box::new(MockAsyncStreamable {}));

        assert_eq!(fetchable_flow.name(), "MockFetcher");
        assert_eq!(streamable_flow.name(), "MockStreamable");
        assert_eq!(async_streamable_flow.name(), "MockAsyncStreamable");
    }

    #[test]
    fn test_flow_format() {
        let fetchable_flow = Flow::Fetchable(Box::new(MockFetcher {}));
        let streamable_flow = Flow::Streamable(Box::new(MockStreamable {}));
        let async_streamable_flow = Flow::AsyncStreamable(Box::new(MockAsyncStreamable {}));

        assert_eq!(fetchable_flow.format(), Format::Asd);
        assert_eq!(streamable_flow.format(), Format::Csv);
        assert_eq!(async_streamable_flow.format(), Format::Json);
    }

    #[tokio::test]
    async fn test_async_streamable_authenticate() {
        let async_streamable_flow = Flow::AsyncStreamable(Box::new(MockAsyncStreamable {}));

        match async_streamable_flow {
            Flow::AsyncStreamable(s) => {
                let token = s.authenticate().await.expect("Failed to authenticate");
                assert_eq!(token, "async_token");
            }
            _ => panic!("Expected AsyncStreamable variant"),
        }
    }

    #[test]
    fn test_fetchable_fetch() {
        let fetchable_flow = Flow::Fetchable(Box::new(MockFetcher {}));
        let (tx, rx) = channel();

        match fetchable_flow {
            Flow::Fetchable(s) => {
                s.fetch(tx, "test_token", "args").expect("Failed to fetch");
                let result = rx.recv().unwrap_or_else(|_| "No data received".to_string());
                assert!(result.contains("test_token"));
                assert!(result.contains("args"));
            }
            _ => panic!("Expected Fetchable variant"),
        }
    }
}
