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

use std::path::PathBuf;

use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigStoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct ConfigStore<T> {
    path: PathBuf,
    data: T,
}

impl<T> ConfigStore<T>
where
    T: Serialize + DeserializeOwned + Default,
{
    pub fn load(path: impl Into<PathBuf>) -> Result<Self, ConfigStoreError> {
        let path = path.into();
        let content = std::fs::read_to_string(&path)?;
        let data = serde_json::from_str(&content)?;

        Ok(Self { path, data })
    }

    pub fn load_or_default(path: impl Into<PathBuf>) -> Result<Self, ConfigStoreError> {
        let path = path.into();
        let data = if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            serde_json::from_str(&content)?
        } else {
            T::default()
        };

        Ok(Self { path, data })
    }

    pub fn save(&self) -> Result<(), ConfigStoreError> {
        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(&self.data)?;
        std::fs::write(&self.path, content)?;

        Ok(())
    }

    pub fn get(&self) -> &T {
        &self.data
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.data
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}
