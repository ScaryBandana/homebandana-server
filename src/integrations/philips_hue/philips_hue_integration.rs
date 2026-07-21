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

use std::time::Duration;

use async_trait::async_trait;
use reqwest::{Certificate, Client};
use serde_json::json;
use tokio::time::sleep;
use uuid::Uuid;

use crate::integrations::{
    integration::Integration,
    integration_error::IntegrationError,
    philips_hue::{
        philips_hue_bridge::PhilipsHueBridge, philips_hue_error::PhilipsHueError,
        v2::philips_hue_v2_bridge_authorization::PhilipsHueV2BridgeAuthorizationResponse,
    },
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
        let client = Client::builder()
            .timeout(if cfg!(test) {
                Duration::from_millis(50)
            } else {
                Duration::from_secs(30)
            })
            .build()?;

        let bridges = client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json::<Vec<PhilipsHueBridge>>()
            .await?;

        match bridges.is_empty() {
            true => Err(PhilipsHueError::NoBridgesDiscovered),
            false => Ok(bridges),
        }
    }

    async fn link_bridge(&self, bridge: &PhilipsHueBridge) -> Result<String, PhilipsHueError> {
        let hue_root_ca_primary = Certificate::from_pem(include_bytes!("certificates/hue_root_ca_primary.pem"))?;
        let hue_root_ca_secondary = Certificate::from_pem(include_bytes!("certificates/hue_root_ca_secondary.pem"))?;

        let client = Client::builder()
            .tls_certs_only([hue_root_ca_primary, hue_root_ca_secondary])
            // This is necessary because the certificate used by the Hue bridge uses the bridge ID as its CN and provides no SAN entries.
            // Connecting via its IP address therefore causes a hostname mismatch error, requiring hostname verification to be disabled.
            .danger_accept_invalid_hostnames(true)
            .timeout(if cfg!(test) {
                Duration::from_millis(50)
            } else {
                Duration::from_secs(30)
            })
            .build()?;

        let url = format!(
            "{}://{}:{}/api",
            if cfg!(test) { "http" } else { "https" },
            bridge.internal_ip_address,
            bridge.port
        );

        let max_attempts = 30;
        for _ in 1..=max_attempts {
            let responses = client
                .post(&url)
                .json(&json!({
                    "devicetype": "homebandana#unknown"
                }))
                .send()
                .await?
                .error_for_status()?
                .json::<Vec<PhilipsHueV2BridgeAuthorizationResponse>>()
                .await?;

            // We only expect one response, but the Hue bridge returns an array of responses, so we take the first one and ignore the rest.
            let first_response = responses.into_iter().next().ok_or(PhilipsHueError::UnexpectedResponse(
                "Response expected to contain at least one element, but was empty".to_string(),
            ))?;

            if let Some(error) = first_response.error {
                if error.kind == 101 {
                    // Link button not pressed, wait and retry.

                    // Don't sleep during tests to speed up test execution.
                    if !cfg!(test) {
                        sleep(Duration::from_secs(1)).await;
                    }

                    continue;
                }

                Err(PhilipsHueError::BridgeLinkingFailed(format!(
                    "{} ({}) at address {})",
                    error.description, error.kind, error.address
                )))?;
            } else if let Some(success) = first_response.success {
                return Ok(success.username);
            } else {
                Err(PhilipsHueError::UnexpectedResponse(
                    "Response expected to contain either 'error' or 'success', but neither was found".to_string(),
                ))?;
            }
        }

        Err(PhilipsHueError::BridgeLinkingTimeout)
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
    use std::time::Duration;

    use httpmock::{
        Method::{GET, POST},
        MockServer,
    };
    use serde_json::json;
    use uuid::Uuid;

    use crate::integrations::philips_hue::{
        philips_hue_bridge::PhilipsHueBridge, philips_hue_error::PhilipsHueError,
        philips_hue_integration::PhilipsHueIntegration,
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

    #[tokio::test]
    async fn test_discover_bridges_http_error_fails() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET).path("/");
            then.status(500);
        });

        let integration = {
            let uuid = Uuid::new_v4();

            PhilipsHueIntegration::new(uuid)
        };

        let result = integration.discover_bridges(&server.base_url()).await;
        mock.assert();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PhilipsHueError::HttpRequestFailed(error) if error.is_status()));
    }

    #[tokio::test]
    async fn test_discover_bridges_invalid_json_fails() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET).path("/");
            then.status(200)
                .header("Content-Type", "application/json")
                .body("invalid json");
        });

        let integration = {
            let uuid = Uuid::new_v4();

            PhilipsHueIntegration::new(uuid)
        };

        let result = integration.discover_bridges(&server.base_url()).await;
        mock.assert();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PhilipsHueError::HttpRequestFailed(error) if error.is_decode()));
    }

    #[tokio::test]
    async fn test_discover_bridges_timeout_fails() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET).path("/");
            then.status(200).delay(Duration::from_millis(100));
        });

        let integration = {
            let uuid = Uuid::new_v4();

            PhilipsHueIntegration::new(uuid)
        };

        let result = integration.discover_bridges(&server.base_url()).await;
        mock.assert();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PhilipsHueError::HttpRequestFailed(error) if error.is_timeout()));
    }

    #[tokio::test]
    async fn test_link_bridge_succeeds() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(POST).path("/api").json_body(json!(
                {
                    "devicetype": "homebandana#unknown",
                }
            ));
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!(
                    [
                        {
                            "success": {
                                "username": "FEDCBA9876543210"
                            }
                        }
                    ]
                ));
        });

        let integration = {
            let uuid = Uuid::new_v4();

            PhilipsHueIntegration::new(uuid)
        };

        let bridge = PhilipsHueBridge {
            id: "0123456789ABCDEF".to_string(),
            internal_ip_address: server.address().ip().to_string(),
            port: server.address().port(),
        };

        let result = integration.link_bridge(&bridge).await;
        mock.assert();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "FEDCBA9876543210");
    }

    #[tokio::test]
    async fn test_link_bridge_error_response_fails() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(POST).path("/api").json_body(json!(
                {
                    "devicetype": "homebandana#unknown",
                }
            ));
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!(
                    [
                        {
                            "error": {
                                "type": 67,
                                "address": "/api",
                                "description": "six seven"
                            }
                        }
                    ]
                ));
        });

        let integration = {
            let uuid = Uuid::new_v4();

            PhilipsHueIntegration::new(uuid)
        };

        let bridge = PhilipsHueBridge {
            id: "0123456789ABCDEF".to_string(),
            internal_ip_address: server.address().ip().to_string(),
            port: server.address().port(),
        };

        let result = integration.link_bridge(&bridge).await;
        mock.assert();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PhilipsHueError::BridgeLinkingFailed(_)));
    }

    #[tokio::test]
    async fn test_link_bridge_link_button_not_pressed_fails() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(POST).path("/api").json_body(json!(
                {
                    "devicetype": "homebandana#unknown",
                }
            ));
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!(
                    [
                        {
                            "error": {
                                "type": 101,
                                "address": "/api",
                                "description": "link button not pressed"
                            }
                        }
                    ]
                ));
        });

        let integration = {
            let uuid = Uuid::new_v4();

            PhilipsHueIntegration::new(uuid)
        };

        let bridge = PhilipsHueBridge {
            id: "0123456789ABCDEF".to_string(),
            internal_ip_address: server.address().ip().to_string(),
            port: server.address().port(),
        };

        let result = integration.link_bridge(&bridge).await;
        mock.assert_calls(30); // Must match max_attempts in link_bridge.
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PhilipsHueError::BridgeLinkingTimeout));
    }

    #[tokio::test]
    async fn test_link_bridge_http_error_fails() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(POST).path("/api").json_body(json!(
                {
                    "devicetype": "homebandana#unknown",
                }
            ));
            then.status(500);
        });

        let integration = {
            let uuid = Uuid::new_v4();

            PhilipsHueIntegration::new(uuid)
        };

        let bridge = PhilipsHueBridge {
            id: "0123456789ABCDEF".to_string(),
            internal_ip_address: server.address().ip().to_string(),
            port: server.address().port(),
        };

        let result = integration.link_bridge(&bridge).await;
        mock.assert();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PhilipsHueError::HttpRequestFailed(error) if error.is_status()));
    }

    #[tokio::test]
    async fn test_link_bridge_invalid_json_fails() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(POST).path("/api").json_body(json!(
                {
                    "devicetype": "homebandana#unknown",
                }
            ));
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!("invalid json"));
        });

        let integration = {
            let uuid = Uuid::new_v4();

            PhilipsHueIntegration::new(uuid)
        };

        let bridge = PhilipsHueBridge {
            id: "0123456789ABCDEF".to_string(),
            internal_ip_address: server.address().ip().to_string(),
            port: server.address().port(),
        };

        let result = integration.link_bridge(&bridge).await;
        mock.assert();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PhilipsHueError::HttpRequestFailed(error) if error.is_decode()));
    }

    #[tokio::test]
    async fn test_link_bridge_timeout_fails() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(POST).path("/api").json_body(json!(
                {
                    "devicetype": "homebandana#unknown",
                }
            ));
            then.status(200).delay(Duration::from_millis(100));
        });

        let integration = {
            let uuid = Uuid::new_v4();

            PhilipsHueIntegration::new(uuid)
        };

        let bridge = PhilipsHueBridge {
            id: "0123456789ABCDEF".to_string(),
            internal_ip_address: server.address().ip().to_string(),
            port: server.address().port(),
        };

        let result = integration.link_bridge(&bridge).await;
        mock.assert();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PhilipsHueError::HttpRequestFailed(error) if error.is_timeout()));
    }

    #[tokio::test]
    async fn test_link_bridge_empty_response_array_fails() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(POST).path("/api").json_body(json!(
                {
                    "devicetype": "homebandana#unknown",
                }
            ));
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!([]));
        });

        let integration = {
            let uuid = Uuid::new_v4();

            PhilipsHueIntegration::new(uuid)
        };

        let bridge = PhilipsHueBridge {
            id: "0123456789ABCDEF".to_string(),
            internal_ip_address: server.address().ip().to_string(),
            port: server.address().port(),
        };

        let result = integration.link_bridge(&bridge).await;
        mock.assert();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PhilipsHueError::UnexpectedResponse(_)));
    }

    #[tokio::test]
    async fn test_link_bridge_response_without_success_or_error_fails() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(POST).path("/api").json_body(json!(
                {
                    "devicetype": "homebandana#unknown",
                }
            ));
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!([{"unexpected": "response"}]));
        });

        let integration = {
            let uuid = Uuid::new_v4();

            PhilipsHueIntegration::new(uuid)
        };

        let bridge = PhilipsHueBridge {
            id: "0123456789ABCDEF".to_string(),
            internal_ip_address: server.address().ip().to_string(),
            port: server.address().port(),
        };

        let result = integration.link_bridge(&bridge).await;
        mock.assert();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PhilipsHueError::UnexpectedResponse(_)));
    }
}
