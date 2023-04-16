// Copyright 2016-2019 Cargo-Bundle developers <https://github.com/burtonageo/cargo-bundle>
// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT
use super::super::common;
use crate::Settings;
use anyhow::Context;
use log::info;
use rpm::signature::pgp::{Signer, Verifier};
use rpm::{self, RPMError, RPMPackage};
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

/// Bundles the project.
pub fn bundle_project(settings: &Settings) -> crate::Result<Vec<PathBuf>> {
  // unimplemented!();
  match rpm_bundle(settings) {
    Ok(path) => Ok(vec![path]),
    Err(e) => Err(crate::Error::BundlerError(anyhow::anyhow!(e.to_string()))),
  }
}

fn rpm_bundle(settings: &Settings) -> anyhow::Result<PathBuf, RPMError> {
    println!("test 1....................");
  let arch = match settings.binary_arch() {
    "x86" => "i386",
    "x86_64" => "amd64",
    // ARM64 is detected differently, armel isn't supported, so armhf is the only reasonable choice here.
    "arm" => "armhf",
    "aarch64" => "arm64",
    other => other,
  };
  let package_base_name = format!(
    "{}_{}_{}",
    settings.main_binary_name(),
    settings.version_string(),
    arch
  );
  let package_name = format!("{}.rpm", package_base_name);
  println!("test 2....................");
  let base_dir = settings.project_out_directory().join("bundle/rpm");
  if base_dir.exists() {
    std::fs::remove_dir_all(&base_dir)
      .map_err(|_| RPMError::Nom(format!("Failed to remove old {}", package_base_name)))?;
  }
  std::fs::create_dir_all(base_dir.clone());

  // let package_dir = base_dir.join(&package_base_name);
  // if package_dir.exists() {
  //   std::fs::remove_dir_all(&package_dir)
  //     .map_err(|_| RPMError::Nom(format!("Failed to remove old {}", package_base_name)))?;
  // }
  let package_path = base_dir.join(&package_name);
  

  // info!(action = "Bundling"; "{} ({})", package_name, package_path.display());

  // let (data_dir) = generate_data(settings, &package_dir)
  //   .map_err(|_| RPMError::Nom("Failed to build data folders and files".to_string()))?;

  // let raw_secret_key = std::fs::read("/path/to/gpg.secret.key")?;
  let mut pkg_builder = rpm::RPMBuilder::new(
    package_base_name.as_str(),
    settings.version_string(),
    "MIT",
    // "x86_64",
    settings.binary_arch(),
    settings.short_description(),
  )
  .compression(rpm::Compressor::from_str("gzip")?);
  // .with_file(
  //     "./awesome-config.toml",
  //     rpm::RPMFileOptions::new("/etc/awesome/config.toml").is_config(),
  // )?
  // file mode is inherited from source file
  println!("test 3....................");

  // for bin in settings.binaries() {
  //   let bin_path = settings.binary_path(bin);
  //   common::copy_file(&bin_path, bin_dir.join(bin.name()))
  //     .with_context(|| format!("Failed to copy binary from {:?}", bin_path))?;
  // }
  //package binary files
  for binary in settings.binaries() {
    pkg_builder = pkg_builder.with_file(
        settings.binary_path(binary),
    //   rpm::RPMFileOptions::new(binary.src_path().unwrap()), // format!("/usr/bin/{}", binary.name())
      rpm::RPMFileOptions::new(format!("/usr/bin/{}", binary.name())),
    )?;
  }
  //package other files
  for (rpm_path, path) in settings.rpm().files.iter() {
    // let rpm_path = if rpm_path.is_absolute() {
    //   rpm_path.strip_prefix("/").unwrap()
    // } else {
    //   rpm_path
    // };
    if path.is_file() {
      println!("{:?} -> {:?}", path, rpm_path);
      pkg_builder = pkg_builder.with_file(
        path,
      rpm::RPMFileOptions::new(rpm_path.to_string_lossy()),
      )?;
      // common::copy_file(path, data_dir.join(rpm_path)).map_err(|e| RPMError::Nom(e.to_string()))?;
    } else {
      // let out_dir = data_dir.join(rpm_path);
      for entry in walkdir::WalkDir::new(path) {
        let entry_path = entry.map_err(|e| RPMError::Nom(e.to_string()))?.into_path();
        if entry_path.is_file() {
          let without_prefix = entry_path.strip_prefix(path).unwrap();
          println!("{:?} -> {:?}", entry_path.clone().to_string_lossy(), rpm_path.join(without_prefix).to_string_lossy());
          pkg_builder = pkg_builder.with_file(
            &entry_path,
          rpm::RPMFileOptions::new(rpm_path.join(without_prefix).to_string_lossy()),
          )?;

          // common::copy_file(&entry_path, out_dir.join(without_prefix))
          //   .map_err(|e| RPMError::Nom(e.to_string()))?;
        }
      }
    }
  }
  // pkg_builder = pkg_builder.with_file(
  //     "./awesome-bin",
  //     rpm::RPMFileOptions::new("/usr/bin/awesome"),
  // )?;
  println!("test 4 ...................");
  //files
  // for (rpm_path, path) in settings.rpm().files.iter() {
  //   let rpm_path = if rpm_path.is_absolute() {
  //     rpm_path.strip_prefix("/").unwrap()
  //   } else {
  //     rpm_path
  //   };
  //   if path.is_file() {
  //     common::copy_file(path, data_dir.join(rpm_path)).map_err(|e| RPMError::Nom(e.to_string()))?;
  //   } else {
  //     let out_dir = data_dir.join(rpm_path);
  //     for entry in walkdir::WalkDir::new(path) {
  //       let entry_path = entry.map_err(|e| RPMError::Nom(e.to_string()))?.into_path();
  //       if entry_path.is_file() {
  //         let without_prefix = entry_path.strip_prefix(path).unwrap();
  //         common::copy_file(&entry_path, out_dir.join(without_prefix))
  //           .map_err(|e| RPMError::Nom(e.to_string()))?;
  //       }
  //     }
  //   }
  // }
  // settings::rpm::pre_install_script(settings
    println!("test 5...................");
  if let Some(prerm_path) = settings.rpm().prerm_path.as_ref() {
    pkg_builder = pkg_builder.pre_uninstall_script(prerm_path);
  }
  // set scripts(post inst)
  if let Some(postinst_path) = settings.rpm().postinst_path.as_ref() {
    let postinst_pathbuf = PathBuf::from(postinst_path);
    if let Ok(body) = std::fs::read_to_string(postinst_pathbuf.clone()) {
      pkg_builder = pkg_builder.post_install_script(body);
    } else {
      return Err(RPMError::Nom(format!("can't read {:?} postinst_path.", postinst_pathbuf)));
    }
  }
  // set scripts(pre rm)
  if let Some(prerm_path) = settings.rpm().prerm_path.as_ref() {
    let prerm_pathbuf = PathBuf::from(prerm_path);
    if let Ok(body) = std::fs::read_to_string(prerm_pathbuf.clone()) {
      pkg_builder = pkg_builder.pre_uninstall_script(body);
    } else {
      return Err(RPMError::Nom(format!("can't read {:?} postinst_path.", prerm_pathbuf)));
    }
  }
  if let Some(copyright) = settings.copyright_string() {
    pkg_builder = pkg_builder.vendor(copyright);
  }
  println!("test 6...................");
  let pkg = pkg_builder
    // .with_file(
    //     "./awesome-config.toml",
    //     // you can set a custom mode and custom user too
    //     rpm::RPMFileOptions::new("/etc/awesome/second.toml")
    //         .mode(0o100744)
    //         .user("hugo"),
    // )?
    // .pre_install_script("echo pre install")
    // .add_changelog_entry("me", "was awesome, eh?", 123123123)
    // .add_changelog_entry("you", "yeah, it was", 12312312)
    // .requires(rpm::Dependency::any("wget"))
    // .vendor("corporation or individual")
    // .url("www.github.com/repo")
    // .vcs("git:repo=example_repo:branch=example_branch:sha=example_sha")
    // .build_and_sign(Signer::load_from_asc_bytes(&raw_secret_key)?);
    .build()?;
  
  let mut f = std::fs::File::create(package_path.clone())?;
  pkg.write(&mut f)?;

  // reading
  // let raw_pub_key = std::fs::read("/path/to/gpg.key.pub")?;
  // let pkg = rpm::RPMPackage::open("test_assets/389-ds-base-devel-1.3.8.4-15.el7.x86_64.rpm")?;

  Ok(package_path)
}

/// Generate the rpm data folders and files.
pub fn generate_data(settings: &Settings, package_dir: &Path) -> crate::Result<(PathBuf)> {
  // Generate data files.
  let data_dir = package_dir.join("data");
  let bin_dir = data_dir.join("usr/bin");

  for bin in settings.binaries() {
    let bin_path = settings.binary_path(bin);
    common::copy_file(&bin_path, bin_dir.join(bin.name()))
      .with_context(|| format!("Failed to copy binary from {:?}", bin_path))?;
  }

  copy_resource_files(settings, &data_dir).with_context(|| "Failed to copy resource files")?;

  settings
    .copy_binaries(&bin_dir)
    .with_context(|| "Failed to copy external binaries")?;

  // let icons =
  //     generate_icon_files(settings, &data_dir).with_context(|| "Failed to create icon files")?;
  // generate_desktop_file(settings, &data_dir).with_context(|| "Failed to create desktop file")?;

  Ok((data_dir))
}

fn copy_resource_files(settings: &Settings, data_dir: &Path) -> crate::Result<()> {
  let resource_dir = data_dir.join("usr/lib").join(settings.main_binary_name());
  settings.copy_resources(&resource_dir)
}
