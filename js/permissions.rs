use std::convert::TryFrom;

use ipnet::IpNet;
use thiserror::Error;
use url::Url;

#[derive(Clone, Debug, Error)]
pub enum PermissionsError {
    #[error("Network destination disallowed")]
    NetAddressDenied,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetHostAndPort {
    pub host: String,
    pub port: Option<u16>,
}

impl NetHostAndPort {
    fn check<T: AsRef<str>>(&self, host: T, port: Option<u16>) -> bool {
        if self.host != host.as_ref() {
            return false;
        }

        match (self.port, port) {
            (Some(a), Some(b)) => a == b,
            (None, _) => true,
            (Some(_), None) => false,
        }
    }
}

impl TryFrom<&str> for NetHostAndPort {
    type Error = url::ParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let (url, port) = if value.contains('/') {
            let url = Url::parse(value)?;
            let port = url.port_or_known_default();
            (url, port)
        } else {
            let url = Url::parse(&format!("http://{}", value))?;
            let port = url.port();
            (url, port)
        };

        Ok(NetHostAndPort {
            host: url
                .host_str()
                .ok_or(url::ParseError::RelativeUrlWithoutBase)?
                .to_string(),
            port,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct Permissions {
    pub allow_relative_urls: bool,

    /// Allow access only to certain domains or IP addresses.
    pub net_allow_list: Vec<NetHostAndPort>,
    /// Block access to certain domains or IP addresses.
    pub net_block_list: Vec<NetHostAndPort>,

    /// Allow access only to certain IP ranges
    pub cidr_allow_list: Vec<IpNet>,

    /// Block access to certain IP ranges. Recommended when hosting public users
    /// to prevent fetch requests to internal network resources.
    pub cidr_block_list: Vec<IpNet>,
}

impl Permissions {
    fn check_host(&self, host: &str, port: Option<u16>) -> Result<(), PermissionsError> {
        if self.net_block_list.iter().any(|hp| hp.check(host, port))
            || (!self.net_allow_list.is_empty()
                && !self.net_allow_list.iter().any(|hp| hp.check(host, port)))
        {
            return Err(PermissionsError::NetAddressDenied);
        }

        Ok(())
    }
}

impl deno_net::NetPermissions for Permissions {
    fn check_net<T: AsRef<str>>(
        &mut self,
        host: &(T, Option<u16>),
        api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        // Check the host. CIDR matching takes place at a lower level.
        self.check_host(host.0.as_ref(), host.1)?;
        Ok(())
    }

    fn check_read(
        &mut self,
        _p: &std::path::Path,
        api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        Ok(())
    }

    fn check_write(
        &mut self,
        _p: &std::path::Path,
        api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        Ok(())
    }
}

impl deno_fetch::FetchPermissions for Permissions {
    fn check_net_url(
        &mut self,
        url: &url::Url,
        api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        let host = match (url.host_str(), self.allow_relative_urls) {
            (Some(host), _) => host,
            (None, true) => return Ok(()),
            (None, false) => return Err(PermissionsError::NetAddressDenied.into()),
        };

        let port = url.port_or_known_default();
        self.check_host(host, port)?;
        Ok(())
    }

    fn check_read(
        &mut self,
        _p: &std::path::Path,
        api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        Ok(())
    }
}

impl deno_web::TimersPermission for Permissions {
    fn allow_hrtime(&mut self) -> bool {
        true
    }

    fn check_unstable(&self, state: &deno_core::OpState, api_name: &'static str) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    mod net_host_and_port {
        use super::*;

        #[test]
        fn parse() {
            assert_eq!(
                NetHostAndPort::try_from("10.2.3.4"),
                Ok(NetHostAndPort {
                    host: "10.2.3.4".to_string(),
                    port: None
                })
            );

            assert_eq!(
                NetHostAndPort::try_from("10.2.3.4:8080"),
                Ok(NetHostAndPort {
                    host: "10.2.3.4".to_string(),
                    port: Some(8080)
                })
            );

            assert_eq!(
                NetHostAndPort::try_from("example.com"),
                Ok(NetHostAndPort {
                    host: "example.com".to_string(),
                    port: None
                })
            );

            assert_eq!(
                NetHostAndPort::try_from("example.com:8080"),
                Ok(NetHostAndPort {
                    host: "example.com".to_string(),
                    port: Some(8080)
                })
            );

            assert_eq!(
                NetHostAndPort::try_from("http://example.com"),
                Ok(NetHostAndPort {
                    host: "example.com".to_string(),
                    port: Some(80)
                })
            );

            assert_eq!(
                NetHostAndPort::try_from("https://example.com"),
                Ok(NetHostAndPort {
                    host: "example.com".to_string(),
                    port: Some(443)
                })
            );

            assert_eq!(
                NetHostAndPort::try_from("http://example.com:8080"),
                Ok(NetHostAndPort {
                    host: "example.com".to_string(),
                    port: Some(8080)
                })
            );

            assert!(NetHostAndPort::try_from("/abc").is_err());
            assert!(NetHostAndPort::try_from(":34").is_err());
        }
    }
}
