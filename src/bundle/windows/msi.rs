// Copyright 2016-2019 Cargo-Bundle developers <https://github.com/burtonageo/cargo-bundle>
// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

mod wix;

use crate::Settings;
use log::warn;

use std::{self, path::PathBuf};

// change by generalworksinc start-------------
const WIX_REQUIRED_FILES_V3: &[&str] = &[
// change by generalworksinc end  -------------
  "candle.exe",
  "candle.exe.config",
  "darice.cub",
  "light.exe",
  "light.exe.config",
  "wconsole.dll",
  "winterop.dll",
  "wix.dll",
  "WixUIExtension.dll",
  "WixUtilExtension.dll",
];
// change by generalworksinc start-------------
const WIX_REQUIRED_FILES_V4: &[&str] = &[
  "wix.exe",
  "wix.dll.config",
  "wix.dll",
  "WixToolset.Core.Native.dll",
  "WixToolset.Core.WindowsInstaller.dll",
  "WixToolset.Data.dll",
  "WixToolset.Dtf.Resources.dll",
  "WixToolset.Extensibility.dll",
  "WixToolset.Versioning.dll",
  "WixToolset.Converters.dll",
  "WixToolset.Core.Burn.dll",
  "WixToolset.Core.dll",
  "WixToolset.Core.ExtensionCache.dll",
];
// change by generalworksinc end  -------------

/// Runs all of the commands to build the MSI installer.
/// Returns a vector of PathBuf that shows where the MSI was created.
pub fn bundle_project(settings: &Settings, updater: bool) -> crate::Result<Vec<PathBuf>> {
// add by generalworksinc start-------------
  let wix_version = match settings.windows().wix.as_ref().and_then(|x| x.version) {
    Some(4) => 4,
    _ => 3,
  };
  println!("wix_version: {}", wix_version);
// add by generalworksinc end  -------------

  let mut wix_path = dirs_next::cache_dir().unwrap();

// change by generalworksinc end  -------------
  wix_path.push("bundler/WixTools/v".to_string() + &wix_version.to_string());
// change by generalworksinc start-------------

  if !wix_path.exists() {
// change by generalworksinc start  -------------
    wix::get_and_extract_wix(&wix_path, wix_version)?;
  } else {
    if wix_version == 4
      && WIX_REQUIRED_FILES_V4
        .iter()
        .any(|p| !wix_path.join(p).exists())
    {
      warn!("WixTools directory is missing some files. Recreating it.");
      std::fs::remove_dir_all(&wix_path)?;
      wix::get_and_extract_wix(&wix_path, wix_version)?;
    } else if WIX_REQUIRED_FILES_V3
      .iter()
      .any(|p| !wix_path.join(p).exists())
    {
      warn!("WixTools directory is missing some files. Recreating it.");
      std::fs::remove_dir_all(&wix_path)?;
      wix::get_and_extract_wix(&wix_path, wix_version)?;
    }
// change by generalworksinc end  -------------
  }

// add by generalworksinc start  -------------
  if wix_version == 4 {
    wix_path.push("tools");
    wix_path.push("net6.0");
    wix_path.push("any");
  }
// add by generalworksinc end    -------------
  wix::build_wix_app_installer(settings, &wix_path, updater)
}
