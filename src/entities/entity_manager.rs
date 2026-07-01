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
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::entities::entity::Entity;

#[derive(Debug, Error)]
pub enum EntityManagerError {
    #[error("An entity with UUID {0} is already registered")]
    AlreadyRegistered(Uuid),
    #[error("No entity with UUID {0} is registered")]
    NotRegistered(Uuid),
}

pub struct EntityManager {
    registry: RwLock<HashMap<Uuid, Arc<dyn Entity>>>,
}

impl EntityManager {
    pub fn new() -> Self {
        Self {
            registry: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(&self, entity: Arc<dyn Entity>) -> Result<(), EntityManagerError> {
        let uuid = entity.uuid();
        let mut registry = self.registry.write().await;

        match registry.entry(uuid) {
            Entry::Occupied(_) => Err(EntityManagerError::AlreadyRegistered(uuid)),
            Entry::Vacant(entry) => {
                entry.insert(entity);

                Ok(())
            }
        }
    }

    pub async fn unregister(&self, uuid: Uuid) -> Result<(), EntityManagerError> {
        let mut registry = self.registry.write().await;

        registry.remove(&uuid).ok_or(EntityManagerError::NotRegistered(uuid))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::entities::{
        entity::Entity,
        entity_manager::{EntityManager, EntityManagerError},
        mock_entity::MockEntity,
    };

    #[tokio::test]
    async fn test_register_succeeds() {
        let manager = EntityManager::new();
        let entity = {
            let uuid = Uuid::new_v4();
            let name = "Test entity".to_string();

            Arc::new(MockEntity::new(uuid, name))
        };

        let result = manager.register(entity).await;
        assert!(result.is_ok());
        assert_eq!(manager.registry.read().await.len(), 1);
    }

    #[tokio::test]
    async fn test_register_already_registered_fails() {
        let manager = EntityManager::new();
        let entity = {
            let uuid = Uuid::new_v4();
            let name = "Test entity".to_string();

            Arc::new(MockEntity::new(uuid, name))
        };
        let _ = manager.register(entity.clone()).await;

        let result = manager.register(entity).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(EntityManagerError::AlreadyRegistered(_))));
        assert_eq!(manager.registry.read().await.len(), 1);
    }

    #[tokio::test]
    async fn test_unregister_succeeds() {
        let manager = EntityManager::new();
        let entity = {
            let uuid = Uuid::new_v4();
            let name = "Test entity".to_string();

            Arc::new(MockEntity::new(uuid, name))
        };
        let _ = manager.register(entity.clone()).await;

        let result = manager.unregister(entity.uuid()).await;
        assert!(result.is_ok());
        assert_eq!(manager.registry.read().await.len(), 0);
    }

    #[tokio::test]
    async fn test_unregister_not_registered_fails() {
        let manager = EntityManager::new();
        let uuid = Uuid::new_v4();

        let result = manager.unregister(uuid).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(EntityManagerError::NotRegistered(_))));
    }
}
