// Copyright 2016-2019 Cargo-Bundle developers <https://github.com/burtonageo/cargo-bundle>
// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

// A macOS application bundle package is laid out like:
//
// foobar.app    # Actually a directory
//     Contents      # A further subdirectory
//         Info.plist     # An xml file containing the app's metadata
//         MacOS          # A directory to hold executable binary files
//             foobar          # The main binary executable of the app
//             foobar_helper   # A helper application, possibly providing a CLI
//         Resources      # Data files such as images, sounds, translations and nib files
//             en.lproj        # Folder containing english translation strings/data
//         Frameworks     # A directory containing private frameworks (shared libraries)
//         ...            # Any other optional files the developer wants to place here
//
// See https://developer.apple.com/go/?id=bundle-structure for a full
// explanation.
//
// Currently, cargo-bundle does not support Frameworks, nor does it support placing arbitrary
// files into the `Contents` directory of the bundle.

use super::{
  super::common::{self, CommandExt},
  icon::create_icns_file,
  sign::{notarize, notarize_auth, sign, NotarizeAuthError, SignTarget},
};
use crate::Settings;

use anyhow::Context;
use log::{info, warn};

use std::{
  ffi::OsStr,
  fs,
  path::{Path, PathBuf},
  process::Command,
};

/// Bundles the project.
/// Returns a vector of PathBuf that shows where the .app was created.
pub fn bundle_project(settings: &Settings) -> crate::Result<Vec<PathBuf>> {
  // we should use the bundle name (App name) as a MacOS standard.
  // version or platform shouldn't be included in the App name.
  let app_product_name = format!("{}.app", settings.product_name());

  let app_bundle_path = settings
    .project_out_directory()
    .join("bundle/macos")
    .join(&app_product_name);

  info!(action = "Bundling"; "{} ({})", app_product_name, app_bundle_path.display());

  if app_bundle_path.exists() {
    fs::remove_dir_all(&app_bundle_path)
      .with_context(|| format!("Failed to remove old {}", app_product_name))?;
  }
  let bundle_directory = app_bundle_path.join("Contents");
  fs::create_dir_all(&bundle_directory).with_context(|| {
    format!(
      "Failed to create bundle directory at {:?}",
      bundle_directory
    )
  })?;

  let resources_dir = bundle_directory.join("Resources");
  let bin_dir = bundle_directory.join("MacOS");
  let mut sign_paths = Vec::new();

  let bundle_icon_file: Option<PathBuf> =
    { create_icns_file(&resources_dir, settings).with_context(|| "Failed to create app icon")? };

  create_info_plist(&bundle_directory, bundle_icon_file, settings)
    .with_context(|| "Failed to create Info.plist")?;

  let framework_paths = copy_frameworks_to_bundle(&bundle_directory, settings)
    .with_context(|| "Failed to bundle frameworks")?;
  sign_paths.extend(
    framework_paths
      .into_iter()
      .filter(|p| {
        let ext = p.extension();
        ext == Some(OsStr::new("framework")) || ext == Some(OsStr::new("dylib"))
      })
      .map(|path| SignTarget {
        path,
        is_an_executable: false,
      }),
  );

  settings.copy_resources(&resources_dir)?;

  let bin_paths = settings
    .copy_binaries(&bin_dir)
    .with_context(|| "Failed to copy external binaries")?;
  sign_paths.extend(bin_paths.into_iter().map(|path| SignTarget {
    path,
    is_an_executable: true,
  }));

  let bin_paths = copy_binaries_to_bundle(&bundle_directory, settings)?;
  sign_paths.extend(bin_paths.into_iter().map(|path| SignTarget {
    path,
    is_an_executable: true,
  }));

  if let Some(identity) = &settings.macos().signing_identity {
    // Sign frameworks and sidecar binaries first, per apple, signing must be done inside out
    // https://developer.apple.com/forums/thread/701514
    sign_paths.push(SignTarget {
      path: app_bundle_path.clone(),
      is_an_executable: true,
    });

    // Remove extra attributes, which could cause codesign to fail
    // https://developer.apple.com/library/archive/qa/qa1940/_index.html
    remove_extra_attr(&app_bundle_path)?;

    // sign application
    sign(sign_paths, identity, settings)?;

    // notarization is required for distribution
    match notarize_auth() {
      Ok(auth) => {
        notarize(app_bundle_path.clone(), auth, settings)?;
      }
      Err(e) => {
        if matches!(e, NotarizeAuthError::MissingTeamId) {
          return Err(anyhow::anyhow!("{e}").into());
        } else {
          warn!("skipping app notarization, {}", e.to_string());
        }
      }
    }
  }

  Ok(vec![app_bundle_path])
}

fn remove_extra_attr(app_bundle_path: &Path) -> crate::Result<()> {
  Command::new("xattr")
    .arg("-cr")
    .arg(app_bundle_path)
    .output_ok()
    .context("failed to remove extra attributes from app bundle")?;
  Ok(())
}

// Copies the app's binaries to the bundle.
fn copy_binaries_to_bundle(
  bundle_directory: &Path,
  settings: &Settings,
) -> crate::Result<Vec<PathBuf>> {
  let mut paths = Vec::new();
  let dest_dir = bundle_directory.join("MacOS");
  for bin in settings.binaries() {
    let bin_path = settings.binary_path(bin);
    let dest_path = dest_dir.join(bin.name());
    common::copy_file(&bin_path, &dest_path)
      .with_context(|| format!("Failed to copy binary from {:?}", bin_path))?;
    paths.push(dest_path);
  }
  Ok(paths)
}

// Creates the Info.plist file.
fn create_info_plist(
  bundle_dir: &Path,
  bundle_icon_file: Option<PathBuf>,
  settings: &Settings,
) -> crate::Result<()> {
  let format = time::format_description::parse("[year][month][day].[hour][minute][second]")
    .map_err(time::error::Error::from)?;
  let build_number = time::OffsetDateTime::now_utc()
    .format(&format)
    .map_err(time::error::Error::from)?;

  let mut plist = plist::Dictionary::new();
  plist.insert("CFBundleDevelopmentRegion".into(), "English".into());
  plist.insert("CFBundleDisplayName".into(), settings.product_name().into());
  plist.insert(
    "CFBundleExecutable".into(),
    settings.main_binary_name().into(),
  );
  if let Some(path) = bundle_icon_file {
    plist.insert(
      "CFBundleIconFile".into(),
      path
        .file_name()
        .expect("No file name")
        .to_string_lossy()
        .into_owned()
        .into(),
    );
  }
  plist.insert(
    "CFBundleIdentifier".into(),
    settings.bundle_identifier().into(),
  );
  plist.insert("CFBundleInfoDictionaryVersion".into(), "6.0".into());
  plist.insert("CFBundleName".into(), settings.product_name().into());
  plist.insert("CFBundlePackageType".into(), "APPL".into());
  plist.insert(
    "CFBundleShortVersionString".into(),
    settings.version_string().into(),
  );
  plist.insert("CFBundleVersion".into(), build_number.into());
  plist.insert("CSResourcesFileMapped".into(), true.into());
  if let Some(category) = settings.app_category() {
    plist.insert(
      "LSApplicationCategoryType".into(),
      category.macos_application_category_type().into(),
    );
  }
  if let Some(version) = settings.macos().minimum_system_version.clone() {
    plist.insert("LSMinimumSystemVersion".into(), version.into());
  }
  plist.insert("LSRequiresCarbon".into(), true.into());
  plist.insert("NSHighResolutionCapable".into(), true.into());
  if let Some(copyright) = settings.copyright_string() {
    plist.insert("NSHumanReadableCopyright".into(), copyright.into());
  }

  if let Some(exception_domain) = settings.macos().exception_domain.clone() {
    let mut security = plist::Dictionary::new();
    let mut domain = plist::Dictionary::new();
    domain.insert("NSExceptionAllowsInsecureHTTPLoads".into(), true.into());
    domain.insert("NSIncludesSubdomains".into(), true.into());

    let mut exception_domains = plist::Dictionary::new();
    exception_domains.insert(exception_domain, domain.into());
    security.insert("NSExceptionDomains".into(), exception_domains.into());
    plist.insert("NSAppTransportSecurity".into(), security.into());
  }

  if let Some(user_plist_path) = &settings.macos().info_plist_path {
    let user_plist = plist::Value::from_file(user_plist_path)?;
    if let Some(dict) = user_plist.into_dictionary() {
      for (key, value) in dict {
        plist.insert(key, value);
      }
    }
  }

  plist::Value::Dictionary(plist).to_file_xml(bundle_dir.join("Info.plist"))?;

  Ok(())
}

// Copies the framework under `{src_dir}/{framework}.framework` to `{dest_dir}/{framework}.framework`.
fn copy_framework_from(dest_dir: &Path, framework: &str, src_dir: &Path) -> crate::Result<bool> {
  let src_name = format!("{}.framework", framework);
  let src_path = src_dir.join(&src_name);
  if src_path.exists() {
    common::copy_dir(&src_path, &dest_dir.join(&src_name))?;
    Ok(true)
  } else {
    Ok(false)
  }
}

// Copies the macOS application bundle frameworks to the .app
fn copy_frameworks_to_bundle(
  bundle_directory: &Path,
  settings: &Settings,
) -> crate::Result<Vec<PathBuf>> {
  let mut paths = Vec::new();

  let frameworks = settings
    .macos()
    .frameworks
    .as_ref()
    .cloned()
    .unwrap_or_default();
  if frameworks.is_empty() {
    return Ok(paths);
  }
  let dest_dir = bundle_directory.join("Frameworks");
  fs::create_dir_all(bundle_directory)
    .with_context(|| format!("Failed to create Frameworks directory at {:?}", dest_dir))?;
  for framework in frameworks.iter() {
    if framework.ends_with(".framework") {
      let src_path = PathBuf::from(framework);
      let src_name = src_path
        .file_name()
        .expect("Couldn't get framework filename");
      let dest_path = dest_dir.join(src_name);
      common::copy_dir(&src_path, &dest_path)?;
      paths.push(dest_path);
      continue;
    } else if framework.ends_with(".dylib") {
      let src_path = PathBuf::from(framework);
      if !src_path.exists() {
        return Err(crate::Error::GenericError(format!(
          "Library not found: {}",
          framework
        )));
      }
      let src_name = src_path.file_name().expect("Couldn't get library filename");
      let dest_path = dest_dir.join(src_name);
      common::copy_file(&src_path, &dest_path)?;
      paths.push(dest_path);
      continue;
    } else if framework.contains('/') {
      return Err(crate::Error::GenericError(format!(
        "Framework path should have .framework extension: {}",
        framework
      )));
    }
    if let Some(home_dir) = dirs_next::home_dir() {
      if copy_framework_from(&dest_dir, framework, &home_dir.join("Library/Frameworks/"))? {
        continue;
      }
    }
    if copy_framework_from(&dest_dir, framework, &PathBuf::from("/Library/Frameworks/"))?
      || copy_framework_from(
        &dest_dir,
        framework,
        &PathBuf::from("/Network/Library/Frameworks/"),
      )?
    {
      continue;
    }
    return Err(crate::Error::GenericError(format!(
      "Could not locate framework: {}",
      framework
    )));
  }
  Ok(paths)
}
