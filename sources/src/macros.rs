//! Define our own macro to simplify the code
//!

/// Simple macro to generate PathBuf from a series of entries
///
#[macro_export]
macro_rules! makepath {
    ($($item:expr),+) => {
        [
        $(PathBuf::from($item),)+
        ]
        .iter()
        .collect()
    };
}

/// Call the HTTP client with the proper arguments
///
/// - unauth call to fetch token by submitting credentials
///
#[macro_export]
macro_rules! http_post {
    ($self:ident, $url:ident, $cred:expr) => {
        $self
            .client
            .clone()
            .post($url)
            .header(
                "user-agent",
                format!("{}/{}", crate_name!(), crate_version!()),
            )
            .header("content-type", "application/json")
            .json($cred)
            .send()
    };
}

/// Call the HTTP client with the proper arguments
///
/// - auth call to fetch token
///
#[macro_export]
macro_rules! http_get_auth {
    ($self:ident, $url:ident, $token:ident) => {
        $self
            .client
            .clone()
            .get($url)
            .header(
                "user-agent",
                format!("{}/{}", crate_name!(), crate_version!()),
            )
            .header("content-type", "application/json")
            .bearer_auth($token)
            .send()
    };
}

/// Call the HTTP client with the proper arguments
///
/// - auth call to fetch data with submitting data
/// - auth call to fetch data
///
#[macro_export]
macro_rules! http_post_auth {
    ($self:ident, $url:ident, $token:ident, $data:expr) => {
        $self
            .client
            .clone()
            .post($url)
            .header(
                "user-agent",
                format!("{}/{}", crate_name!(), crate_version!()),
            )
            .header("content-type", "application/json")
            .bearer_auth($token)
            .json($data)
            .send()
    };
    ($self:ident, $url:ident, $token:ident) => {
        $self
            .client
            .clone()
            .post($url)
            .header(
                "user-agent",
                format!("{}/{}", crate_name!(), crate_version!()),
            )
            .header("content-type", "application/json")
            .bearer_auth($token)
            .send()
    };
}

/// Call the HTTP client with the proper arguments for BASIC authentication
///
/// - auth call to fetch data with submitting data
/// - auth call to fetch data
///
#[macro_export]
macro_rules! http_get_basic {
    ($self:ident, $url:ident, $user:ident, $pwd:ident, $data:expr) => {
        $self
            .client
            .clone()
            .get($url)
            .basic_auth($user, Some($pwd))
            .header(
                "user-agent",
                format!("{}/{}", crate_name!(), crate_version!()),
            )
            .header("content-type", "application/json")
            .json($data)
            .send()
    };
    ($self:ident, $url:ident, $user:ident, $pwd:ident) => {
        $self
            .client
            .clone()
            .get($url)
            .basic_auth($user, Some($pwd))
            .header(
                "user-agent",
                format!("{}/{}", crate_name!(), crate_version!()),
            )
            .header("content-type", "application/json")
            .send()
    };
}
