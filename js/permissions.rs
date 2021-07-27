use std::convert::TryFrom;

use ipnet::IpNet;
use thiserror::Error;
use url::Url;

#[derive(Clone, Debug, Error)]
pub enum PermissionsError {
    #[error("Network destination disallowed")]
    NetAddressDenied,
}

#[derive(Clone, Debug)]
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
        let url = Url::parse(value)?;
        Ok(NetHostAndPort {
            host: url.host_str().unwrap().to_string(), // TODO no unwrap
            port: url.port(),
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
            return Err(PermissionsError::NetAddressDenied.into());
        }

        Ok(())
    }
}

impl deno_net::NetPermissions for Permissions {
    fn check_net<T: AsRef<str>>(
        &mut self,
        host: &(T, Option<u16>),
    ) -> Result<(), deno_core::error::AnyError> {
        // Check the host. CIDR matching takes place at a lower level.
        self.check_host(host.0.as_ref(), host.1)?;
        Ok(())
    }

    fn check_read(&mut self, _p: &std::path::Path) -> Result<(), deno_core::error::AnyError> {
        Ok(())
    }

    fn check_write(&mut self, _p: &std::path::Path) -> Result<(), deno_core::error::AnyError> {
        Ok(())
    }
}

impl deno_fetch::FetchPermissions for Permissions {
    fn check_net_url(&mut self, url: &url::Url) -> Result<(), deno_core::error::AnyError> {
        let host = match (url.host_str(), self.allow_relative_urls) {
            (Some(host), _) => host,
            (None, true) => return Ok(()),
            (None, false) => return Err(PermissionsError::NetAddressDenied.into()),
        };

        let port = url.port_or_known_default();
        self.check_host(host, port)?;
        Ok(())
    }

    fn check_read(&mut self, _p: &std::path::Path) -> Result<(), deno_core::error::AnyError> {
        Ok(())
    }
}
