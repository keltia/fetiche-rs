use thiserror::Error;

/// Custom error type for the access module, allow us to differentiate between errors.
///
#[derive(Debug, Error)]
pub enum AccessError {
    #[error("Bad configuration parameter: {0}")]
    BadParam(String),
    #[error("No such site {0}")]
    UnknownSite(String),
    #[error("Invalid site {0}")]
    InvalidSite(String),
    #[error("Bad Filter.")]
    BadFilter,
    #[error("Bad Proxy String {0}.")]
    BadProxyString(String),
    #[error("Proxy Connect Failed.")]
    ProxyConnectFailed,
    #[error("TLS Connect Failed: {0}.")]
    TlsConnectFailed(String),
}

#[derive(Debug, Error)]
pub enum DataError {
    #[error("Invalid packet received, can not decode.")]
    BadPacketData,
}

#[derive(Debug, Error)]
pub enum ParamError {
    #[error("No stats actor configured, exiting.")]
    NoStatsActor,
    #[error("No address given.")]
    NoAddrGiven,
    #[error("No auth configured for {0}")]
    NoAuthConfigured(String),
}
