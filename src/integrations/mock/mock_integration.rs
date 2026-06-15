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
    integration::Integration, integration_error::IntegrationError, mock::mock_integration_error::MockIntegrationError,
};

pub struct MockIntegration {
    uuid: Uuid,
    start_returns_error: bool,
    stop_returns_error: bool,
}

impl MockIntegration {
    pub fn new(uuid: Uuid, start_returns_error: bool, stop_returns_error: bool) -> Self {
        Self {
            uuid,
            start_returns_error,
            stop_returns_error,
        }
    }
}

#[async_trait]
impl Integration for MockIntegration {
    fn uuid(&self) -> Uuid {
        self.uuid
    }

    async fn start(&self) -> Result<(), IntegrationError> {
        match self.start_returns_error {
            true => Err(MockIntegrationError::StartReturnsError)?,
            false => Ok(()),
        }
    }

    async fn stop(&self) -> Result<(), IntegrationError> {
        match self.stop_returns_error {
            true => Err(MockIntegrationError::StopReturnsError)?,
            false => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::integrations::{
        integration::Integration,
        integration_error::IntegrationError,
        mock::{mock_integration::MockIntegration, mock_integration_error::MockIntegrationError},
    };

    #[tokio::test]
    async fn test_start_succeeds() {
        let uuid = Uuid::new_v4();
        let start_returns_error = false;
        let stop_returns_error = false;
        let integration = MockIntegration::new(uuid, start_returns_error, stop_returns_error);

        let result = integration.start().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_start_fails() {
        let uuid = Uuid::new_v4();
        let start_returns_error = true;
        let stop_returns_error = false;
        let integration = MockIntegration::new(uuid, start_returns_error, stop_returns_error);

        let result = integration.start().await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            IntegrationError::MockIntegrationError(MockIntegrationError::StartReturnsError)
        ));
    }

    #[tokio::test]
    async fn test_stop_succeeds() {
        let uuid = Uuid::new_v4();
        let start_returns_error = false;
        let stop_returns_error = false;
        let integration = MockIntegration::new(uuid, start_returns_error, stop_returns_error);

        let result = integration.stop().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stop_fails() {
        let uuid = Uuid::new_v4();
        let start_returns_error = false;
        let stop_returns_error = true;
        let integration = MockIntegration::new(uuid, start_returns_error, stop_returns_error);

        let result = integration.stop().await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            IntegrationError::MockIntegrationError(MockIntegrationError::StopReturnsError)
        ));
    }
}
