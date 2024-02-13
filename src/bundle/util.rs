use std::fmt::Display;
use serde::{Serialize, Deserialize, Serializer, Deserializer, de::Error as DeError};



/// A bundle referenced by tauri-bundler.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename_all = "lowercase"))]
pub enum BundleType {
  /// The debian bundle (.deb).
  Deb,
  /// The AppImage bundle (.appimage).
  AppImage,
  /// The Microsoft Installer bundle (.msi).
  Msi,
  /// The NSIS bundle (.exe).
  Nsis,
  /// The macOS application bundle (.app).
  App,
  /// The Apple Disk Image bundle (.dmg).
  Dmg,
  /// The Apple Package bundle (.pkg).
  Pkg,
  /// The Tauri updater bundle.
  Updater,
}

impl Display for BundleType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{}",
      match self {
        Self::Deb => "deb",
        Self::AppImage => "appimage",
        Self::Msi => "msi",
        Self::Nsis => "nsis",
        Self::App => "app",
        Self::Dmg => "dmg",
        Self::Pkg => "pkg",
        Self::Updater => "updater",
      }
    )
  }
}

impl Serialize for BundleType {
  fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_str(self.to_string().as_ref())
  }
}

impl<'de> Deserialize<'de> for BundleType {
  fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let s = String::deserialize(deserializer)?;
    match s.to_lowercase().as_str() {
      "deb" => Ok(Self::Deb),
      "appimage" => Ok(Self::AppImage),
      "msi" => Ok(Self::Msi),
      "nsis" => Ok(Self::Nsis),
      "app" => Ok(Self::App),
      "dmg" => Ok(Self::Dmg),
      "updater" => Ok(Self::Updater),
      _ => Err(DeError::custom(format!("unknown bundle target '{s}'"))),
    }
  }
}
