// Copyright 2026 ScaryBandana
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use async_trait::async_trait;
use uuid::Uuid;

use crate::integrations::{
    integration::Integration,
    integration_error::IntegrationError,
    philips_hue::{philips_hue_bridge::PhilipsHueBridge, philips_hue_error::PhilipsHueError},
};

pub struct PhilipsHueIntegration {
    uuid: Uuid,
}

impl PhilipsHueIntegration {
    #[rustfmt::skip]
    pub fn new(uuid: Uuid) -> Self {
        Self {
            uuid,
        }
    }

    async fn discover_bridges(&self, url: &str) -> Result<Vec<PhilipsHueBridge>, PhilipsHueError> {
        let bridges = reqwest::get(url).await?.json::<Vec<PhilipsHueBridge>>().await?;

        match bridges.is_empty() {
            true => Err(PhilipsHueError::NoBridgesDiscovered),
            false => Ok(bridges),
        }
    }
}

#[async_trait]
impl Integration for PhilipsHueIntegration {
    fn uuid(&self) -> Uuid {
        self.uuid
    }

    async fn start(&self) -> Result<(), IntegrationError> {
        Ok(())
    }

    async fn stop(&self) -> Result<(), IntegrationError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use httpmock::{Method::GET, MockServer};
    use serde_json::json;
    use uuid::Uuid;

    use crate::integrations::philips_hue::{
        philips_hue_error::PhilipsHueError, philips_hue_integration::PhilipsHueIntegration,
    };

    #[tokio::test]
    async fn test_discover_single_bridge_succeeds() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET).path("/");
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!(
                    [
                        {
                            "id": "0123456789ABCDEF",
                            "internalipaddress": "1.2.3.4",
                            "port": 443
                        }
                    ]
                ));
        });

        let uuid = Uuid::new_v4();
        let integration = PhilipsHueIntegration::new(uuid);

        let result = integration.discover_bridges(&server.base_url()).await;
        mock.assert();
        assert!(result.is_ok());

        let bridges = result.unwrap();
        assert_eq!(bridges.len(), 1);

        assert_eq!(bridges[0].id, "0123456789ABCDEF");
        assert_eq!(bridges[0].internal_ip_address, "1.2.3.4");
        assert_eq!(bridges[0].port, 443);
    }

    #[tokio::test]
    async fn test_discover_multiple_bridges_succeeds() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET).path("/");
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!(
                    [
                        // First bridge
                        {
                            "id": "0123456789ABCDEF",
                            "internalipaddress": "1.2.3.4",
                            "port": 443
                        },
                        // Second bridge
                        {
                            "id": "ABCDEF0123456789",
                            "internalipaddress": "5.6.7.8",
                            "port": 334
                        },
                    ]
                ));
        });

        let uuid = Uuid::new_v4();
        let integration = PhilipsHueIntegration::new(uuid);

        let result = integration.discover_bridges(&server.base_url()).await;
        mock.assert();
        assert!(result.is_ok());

        let bridges = result.unwrap();
        assert_eq!(bridges.len(), 2);

        // First bridge
        assert_eq!(bridges[0].id, "0123456789ABCDEF");
        assert_eq!(bridges[0].internal_ip_address, "1.2.3.4");
        assert_eq!(bridges[0].port, 443);

        // Second bridge
        assert_eq!(bridges[1].id, "ABCDEF0123456789");
        assert_eq!(bridges[1].internal_ip_address, "5.6.7.8");
        assert_eq!(bridges[1].port, 334);
    }

    #[tokio::test]
    async fn test_discover_no_bridge_fails() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET).path("/");
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!([]));
        });

        let uuid = Uuid::new_v4();
        let integration = PhilipsHueIntegration::new(uuid);

        let result = integration.discover_bridges(&server.base_url()).await;
        mock.assert();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PhilipsHueError::NoBridgesDiscovered));
    }
}
