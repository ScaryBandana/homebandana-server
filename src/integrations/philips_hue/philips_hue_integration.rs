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

use crate::integrations::{integration::Integration, integration_error::IntegrationError};

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
