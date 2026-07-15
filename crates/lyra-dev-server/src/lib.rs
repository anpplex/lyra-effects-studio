#![doc = "Authenticated loopback Dev Bridge server for Lyra Effects Studio."]

mod diagnostic;
mod server;
mod token;

use std::fmt;
use std::net::SocketAddr;

pub use diagnostic::ServerDiagnostic;
pub use server::DevServer;

use token::BridgeToken;

/// Provisioning data for one authenticated loopback server instance.
pub struct DevServerEndpoint {
    address: SocketAddr,
    token: BridgeToken,
}

impl DevServerEndpoint {
    pub(crate) const fn new(address: SocketAddr, token: BridgeToken) -> Self {
        Self { address, token }
    }

    #[must_use]
    /// Returns the IPv4 loopback listener address.
    pub const fn address(&self) -> SocketAddr {
        self.address
    }

    #[must_use]
    /// Returns the only device-facing route on this server instance.
    pub fn hello_url(&self) -> String {
        format!("http://{}/v1/hello", self.address)
    }

    #[must_use]
    /// Returns the bearer value for trusted device provisioning.
    ///
    /// Callers must not display, log or persist this secret.
    pub fn authorization_value(&self) -> String {
        self.token.authorization_value()
    }

    #[cfg(test)]
    fn new_for_test() -> Self {
        Self {
            address: SocketAddr::from(([127, 0, 0, 1], 32_768)),
            token: BridgeToken::from_bytes([0xa5; 32]),
        }
    }
}

impl fmt::Debug for DevServerEndpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DevServerEndpoint")
            .field("address", &self.address)
            .field("token", &"<redacted>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn endpoint_uses_loopback_and_redacts_its_token() {
        let endpoint = super::DevServerEndpoint::new_for_test();
        assert_eq!(endpoint.address().ip().to_string(), "127.0.0.1");
        assert!(endpoint.address().port() > 0);
        assert_eq!(
            endpoint.hello_url(),
            format!("http://{}/v1/hello", endpoint.address())
        );
        let authorization = endpoint.authorization_value();
        assert!(authorization.starts_with("Bearer "));
        assert_eq!(authorization.len(), 71);
        assert!(!format!("{endpoint:?}").contains(&authorization));
    }
}
