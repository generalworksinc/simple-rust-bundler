//Add GeneralworksInc Start

use super::{app, icon::create_icns_file};
use crate::{
  bundle::{common::CommandExt, Bundle},
  PackageType, Settings,
};

use anyhow::Context;
use log::info;

use std::{
  env,
  fs::{self, write},
  path::PathBuf,
  process::{Command, Stdio},
};

pub struct Bundled {
  pub pkg: Vec<PathBuf>,
  pub app: Vec<PathBuf>,
}

/// Bundles the project.
/// Returns a vector of PathBuf that shows where the DMG was created.
pub fn bundle_project(settings: &Settings, bundles: &[Bundle]) -> crate::Result<Bundled> {
  // generate the .app bundle if needed
  let app_bundle_paths = if !bundles
    .iter()
    .any(|bundle| bundle.package_type == PackageType::Pkg)
  {
    app::bundle_project(settings)?
  } else {
    Vec::new()
  };

  // get the target path
  let output_path = settings.project_out_directory().join("bundle/pkg");
  let package_base_name = format!(
    "{}_{}_{}",
    settings.main_binary_name(),
    settings.version_string(),
    match settings.binary_arch() {
      "x86_64" => "x64",
      other => other,
    }
  );
  let pkg_name = format!("{}.pkg", &package_base_name);
  let pkg_path = output_path.join(&pkg_name);

  // let product_name = settings.main_binary_name();
  let product_name = settings.product_name();
  let bundle_file_name = format!("{}.app", product_name);
  let bundle_dir = settings.project_out_directory().join("bundle/macos");

  // let support_directory_path = output_path.join("support");
  if output_path.exists() {
    fs::remove_dir_all(&output_path)
      .with_context(|| format!("Failed to remove old {}", pkg_name))?;
  }
  fs::create_dir_all(&output_path).with_context(|| {
    format!(
      "Failed to create output directory at {:?}",
      output_path
    )
  })?;

  // fs::create_dir_all(&support_directory_path).with_context(|| {
  //   format!(
  //     "Failed to create output directory at {:?}",
  //     support_directory_path
  //   )
  // })?;

  // create paths for script
  let bundle_script_path = output_path.join("bundle_pkg.sh");

  info!(action = "Bundling"; "{} ({})", pkg_name, pkg_path.display());

  // write the scripts
  write(
    &bundle_script_path,
    include_str!("templates/pkg/bundle_pkg"),
  )?;

  // chmod script for execution
  Command::new("chmod")
    .arg("777")
    .arg(&bundle_script_path)
    .current_dir(&output_path)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .output()
    .expect("Failed to chmod script");
  Command::new("chmod")
    .arg("-rf")
    .arg("777")
    .arg(&bundle_dir.join("*"))
    .current_dir(&output_path)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .output()
    .expect("Failed to chmod macos app bundle script");
    
  let mut args = vec![
    bundle_dir.to_string_lossy().to_string(),
    bundle_file_name.clone(),
    pkg_name.clone(),
    //identifier
    //script name
  ];
  
  let pkg_settings = settings.pkg();
  if let Some(x) = pkg_settings.identifier.clone() {
    args.push(x);
  } else {
    args.push("com.example.app".to_string());
  }
  if let Some(x) = pkg_settings.postinst_dir_path.clone() {
    args.push(x);
  }
  
  info!(action = "Running"; "bundle_pkg.sh");

  println!("bundle_script_path: {:?} {:?}", bundle_script_path, args);
  // execute the bundle script
  Command::new(&bundle_script_path)
    .current_dir(bundle_dir.clone())
    .args(args)
    // .args(vec![dmg_name.as_str(), bundle_file_name.as_str()])
    .output_ok()
    .context("error running bundle_pkg.sh")?;

  fs::rename(bundle_dir.join(pkg_name), pkg_path.clone())?;

  Ok(Bundled {
    pkg: vec![pkg_path],
    app: app_bundle_paths,
  })
}
//Add GeneralworksInc End