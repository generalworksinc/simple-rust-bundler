// Copyright 2016-2019 Cargo-Bundle developers <https://github.com/burtonageo/cargo-bundle>
// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

mod wix;

use crate::Settings;

use std::{self, path::PathBuf};

/// Runs all of the commands to build the MSI installer.
/// Returns a vector of PathBuf that shows where the MSI was created.
pub fn bundle_project(settings: &Settings, updater: bool) -> crate::Result<Vec<PathBuf>> {
  wix::build_wix_app_installer(settings, updater)
}
