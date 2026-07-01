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

use uuid::Uuid;

use crate::entities::entity::Entity;

pub struct MockEntity {
    uuid: Uuid,
    name: String,
}

impl MockEntity {
    #[rustfmt::skip]
    pub fn new(uuid: Uuid, name: String) -> Self {
        Self {
            uuid,
            name
        }
    }
}

impl Entity for MockEntity {
    fn uuid(&self) -> Uuid {
        self.uuid
    }

    fn name(&self) -> &str {
        &self.name
    }
}
