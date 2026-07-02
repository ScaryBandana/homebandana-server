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

use thiserror::Error;

use crate::integrations::{
    mock::mock_integration_error::MockIntegrationError, philips_hue::philips_hue_error::PhilipsHueError,
};

#[derive(Debug, Error)]
pub enum IntegrationError {
    #[error(transparent)]
    MockIntegrationError(#[from] MockIntegrationError),
    #[error(transparent)]
    PhilipsHueError(#[from] PhilipsHueError),
}
