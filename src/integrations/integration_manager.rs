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

use std::{
    collections::{HashMap, hash_map::Entry},
    sync::Arc,
};

use thiserror::Error;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::integrations::{integration::Integration, integration_error::IntegrationError};

pub struct IntegrationEntry {
    op_lock: Mutex<()>,
    is_started: Mutex<bool>,
    integration: Arc<dyn Integration>,
}

#[derive(Debug, Error)]
pub enum IntegrationManagerError {
    #[error("An integration with UUID {0} is already registered")]
    AlreadyRegistered(Uuid),
    #[error("No integration with UUID {0} is registered")]
    NotRegistered(Uuid),
    #[error("Failed to start integration {0}: {1}")]
    StartFailed(Uuid, #[source] IntegrationError),
    #[error("Failed to stop integration {0}: {1}")]
    StopFailed(Uuid, #[source] IntegrationError),
}

pub struct IntegrationManager {
    registry: RwLock<HashMap<Uuid, Arc<IntegrationEntry>>>,
}

impl IntegrationManager {
    pub fn new() -> Self {
        Self {
            registry: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(&self, integration: Arc<dyn Integration>) -> Result<(), IntegrationManagerError> {
        let uuid = integration.uuid();
        let mut registry = self.registry.write().await;

        match registry.entry(uuid) {
            Entry::Occupied(_) => Err(IntegrationManagerError::AlreadyRegistered(uuid)),
            Entry::Vacant(entry) => {
                entry.insert(Arc::new(IntegrationEntry {
                    op_lock: Mutex::new(()),
                    is_started: Mutex::new(false),
                    integration,
                }));

                Ok(())
            }
        }
    }

    pub async fn unregister(&self, uuid: Uuid) -> Result<(), IntegrationManagerError> {
        let entry = {
            let registry = self.registry.read().await;

            registry
                .get(&uuid)
                .cloned()
                .ok_or(IntegrationManagerError::NotRegistered(uuid))?
        };

        let _op_lock = entry.op_lock.lock().await;

        {
            let is_started = entry.is_started.lock().await;

            if *is_started {
                drop(is_started);

                if let Err(e) = entry.integration.stop().await {
                    return Err(IntegrationManagerError::StopFailed(uuid, e));
                }

                let mut is_started = entry.is_started.lock().await;
                *is_started = false;
            }
        }

        let mut registry = self.registry.write().await;
        registry
            .remove(&uuid)
            .ok_or(IntegrationManagerError::NotRegistered(uuid))?;

        Ok(())
    }

    pub async fn start(&self, uuid: Uuid) -> Result<(), IntegrationManagerError> {
        let entry = {
            let registry = self.registry.read().await;

            registry
                .get(&uuid)
                .cloned()
                .ok_or(IntegrationManagerError::NotRegistered(uuid))?
        };

        let _op_lock = entry.op_lock.lock().await;

        {
            let is_started = entry.is_started.lock().await;

            if *is_started {
                // Idempotent start
                return Ok(());
            }
        }

        if let Err(e) = entry.integration.start().await {
            return Err(IntegrationManagerError::StartFailed(uuid, e));
        }

        {
            let mut is_started = entry.is_started.lock().await;

            *is_started = true;
        }

        Ok(())
    }

    pub async fn stop(&self, uuid: Uuid) -> Result<(), IntegrationManagerError> {
        let entry = {
            let registry = self.registry.read().await;

            registry
                .get(&uuid)
                .cloned()
                .ok_or(IntegrationManagerError::NotRegistered(uuid))?
        };

        let _op_lock = entry.op_lock.lock().await;

        {
            let is_started = entry.is_started.lock().await;

            if !*is_started {
                // Idempotent stop
                return Ok(());
            }
        }

        if let Err(e) = entry.integration.stop().await {
            return Err(IntegrationManagerError::StopFailed(uuid, e));
        }

        {
            let mut is_started = entry.is_started.lock().await;

            *is_started = false;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::integrations::{
        integration::Integration,
        integration_manager::{IntegrationManager, IntegrationManagerError},
        mock::mock_integration::MockIntegration,
    };

    #[tokio::test]
    async fn test_register_succeeds() {
        let integration = {
            let uuid = Uuid::new_v4();
            let start_returns_error = false;
            let stop_returns_error = false;

            Arc::new(MockIntegration::new(uuid, start_returns_error, stop_returns_error))
        };
        let manager = IntegrationManager::new();

        let result = manager.register(integration).await;
        assert!(result.is_ok());
        assert_eq!(manager.registry.read().await.len(), 1);
    }

    #[tokio::test]
    async fn test_register_already_registered_fails() {
        let integration = {
            let uuid = Uuid::new_v4();
            let start_returns_error = false;
            let stop_returns_error = false;

            Arc::new(MockIntegration::new(uuid, start_returns_error, stop_returns_error))
        };
        let manager = IntegrationManager::new();
        let _ = manager.register(integration.clone()).await;

        let result = manager.register(integration).await;
        assert!(result.is_err());
        assert!(matches!(
            result.err().unwrap(),
            IntegrationManagerError::AlreadyRegistered(_)
        ));
        assert_eq!(manager.registry.read().await.len(), 1);
    }

    #[tokio::test]
    async fn test_unregister_succeeds() {
        let integration = {
            let uuid = Uuid::new_v4();
            let start_returns_error = false;
            let stop_returns_error = false;

            Arc::new(MockIntegration::new(uuid, start_returns_error, stop_returns_error))
        };
        let manager = IntegrationManager::new();
        let _ = manager.register(integration.clone()).await;

        let result = manager.unregister(integration.uuid()).await;
        assert!(result.is_ok());
        assert_eq!(manager.registry.read().await.len(), 0);
    }

    #[tokio::test]
    async fn test_unregister_not_registered_fails() {
        let uuid = Uuid::new_v4();
        let manager = IntegrationManager::new();

        let result = manager.unregister(uuid).await;
        assert!(result.is_err());
        assert!(matches!(
            result.err().unwrap(),
            IntegrationManagerError::NotRegistered(_)
        ));
        assert_eq!(manager.registry.read().await.len(), 0);
    }

    #[tokio::test]
    async fn test_unregister_started_integration_succeeds() {
        let integration = {
            let uuid = Uuid::new_v4();
            let start_returns_error = false;
            let stop_returns_error = false;

            Arc::new(MockIntegration::new(uuid, start_returns_error, stop_returns_error))
        };
        let manager = IntegrationManager::new();
        let _ = manager.register(integration.clone()).await;
        let _ = manager.start(integration.uuid()).await;

        let result = manager.unregister(integration.uuid()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_unregister_started_integration_fails() {
        let integration = {
            let uuid = Uuid::new_v4();
            let start_returns_error = false;
            let stop_returns_error = true;

            Arc::new(MockIntegration::new(uuid, start_returns_error, stop_returns_error))
        };
        let manager = IntegrationManager::new();
        let _ = manager.register(integration.clone()).await;
        let _ = manager.start(integration.uuid()).await;

        let result = manager.unregister(integration.uuid()).await;
        assert!(result.is_err());
        assert!(matches!(
            result.err().unwrap(),
            IntegrationManagerError::StopFailed(_, _)
        ));
    }

    #[tokio::test]
    async fn test_start_succeeds() {
        let integration = {
            let uuid = Uuid::new_v4();
            let start_returns_error = false;
            let stop_returns_error = false;

            Arc::new(MockIntegration::new(uuid, start_returns_error, stop_returns_error))
        };
        let manager = IntegrationManager::new();
        let _ = manager.register(integration.clone()).await;

        let result = manager.start(integration.uuid()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_start_fails() {
        let integration = {
            let uuid = Uuid::new_v4();
            let start_returns_error = true;
            let stop_returns_error = false;

            Arc::new(MockIntegration::new(uuid, start_returns_error, stop_returns_error))
        };
        let manager = IntegrationManager::new();
        let _ = manager.register(integration.clone()).await;

        let result = manager.start(integration.uuid()).await;
        assert!(result.is_err());
        assert!(matches!(
            result.err().unwrap(),
            IntegrationManagerError::StartFailed(_, _)
        ));
    }

    #[tokio::test]
    async fn test_start_not_registered_fails() {
        let uuid = Uuid::new_v4();
        let manager = IntegrationManager::new();

        let result = manager.start(uuid).await;
        assert!(result.is_err());
        assert!(matches!(
            result.err().unwrap(),
            IntegrationManagerError::NotRegistered(_)
        ));
    }

    #[tokio::test]
    async fn test_stop_succeeds() {
        let integration = {
            let uuid = Uuid::new_v4();
            let start_returns_error = false;
            let stop_returns_error = false;

            Arc::new(MockIntegration::new(uuid, start_returns_error, stop_returns_error))
        };
        let manager = IntegrationManager::new();
        let _ = manager.register(integration.clone()).await;
        let _ = manager.start(integration.uuid()).await;

        let result = manager.stop(integration.uuid()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stop_fails() {
        let integration = {
            let uuid = Uuid::new_v4();
            let start_returns_error = false;
            let stop_returns_error = true;

            Arc::new(MockIntegration::new(uuid, start_returns_error, stop_returns_error))
        };
        let manager = IntegrationManager::new();
        let _ = manager.register(integration.clone()).await;
        let _ = manager.start(integration.uuid()).await;

        let result = manager.stop(integration.uuid()).await;
        assert!(result.is_err());
        assert!(matches!(
            result.err().unwrap(),
            IntegrationManagerError::StopFailed(_, _)
        ));
    }

    #[tokio::test]
    async fn test_stop_not_registered_fails() {
        let uuid = Uuid::new_v4();
        let manager = IntegrationManager::new();

        let result = manager.stop(uuid).await;
        assert!(result.is_err());
        assert!(matches!(
            result.err().unwrap(),
            IntegrationManagerError::NotRegistered(_)
        ));
    }
}
