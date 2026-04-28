use std::{
  collections::HashMap,
  path::{Component, Path, PathBuf},
};

pub fn display_path<P: AsRef<Path>>(p: P) -> String {
  dunce::simplified(&p.as_ref().components().collect::<PathBuf>())
    .display()
    .to_string()
}

pub fn resource_relpath(path: &Path) -> PathBuf {
  let mut dest = PathBuf::new();
  for component in path.components() {
    match component {
      Component::Prefix(_) => {}
      Component::RootDir => dest.push("_root_"),
      Component::CurDir => {}
      Component::ParentDir => dest.push("_up_"),
      Component::Normal(string) => dest.push(string),
    }
  }
  dest
}

pub fn external_binaries(external_binaries: &[String], target_triple: &str) -> Vec<String> {
  external_binaries
    .iter()
    .map(|path| {
      format!(
        "{}-{}{}",
        path,
        target_triple,
        if target_triple.contains("windows") {
          ".exe"
        } else {
          ""
        }
      )
    })
    .collect()
}

enum PatternIter<'a> {
  Slice(std::slice::Iter<'a, String>),
  Map(std::collections::hash_map::Iter<'a, String, String>),
}

pub struct ResourcePaths<'a> {
  iter: ResourcePathsIter<'a>,
}

impl<'a> ResourcePaths<'a> {
  pub fn new(patterns: &'a [String], allow_walk: bool) -> ResourcePaths<'a> {
    ResourcePaths {
      iter: ResourcePathsIter {
        pattern_iter: PatternIter::Slice(patterns.iter()),
        glob_iter: None,
        walk_iter: None,
        allow_walk,
        current_pattern: None,
        current_dest: None,
      },
    }
  }

  pub fn from_map(patterns: &'a HashMap<String, String>, allow_walk: bool) -> ResourcePaths<'a> {
    ResourcePaths {
      iter: ResourcePathsIter {
        pattern_iter: PatternIter::Map(patterns.iter()),
        glob_iter: None,
        walk_iter: None,
        allow_walk,
        current_pattern: None,
        current_dest: None,
      },
    }
  }

  pub fn iter(self) -> ResourcePathsIter<'a> {
    self.iter
  }
}

pub struct ResourcePathsIter<'a> {
  pattern_iter: PatternIter<'a>,
  glob_iter: Option<glob::Paths>,
  walk_iter: Option<walkdir::IntoIter>,
  allow_walk: bool,
  current_pattern: Option<PathBuf>,
  current_dest: Option<PathBuf>,
}

pub struct Resource {
  path: PathBuf,
  target: PathBuf,
}

impl Resource {
  pub fn path(&self) -> &Path {
    &self.path
  }

  pub fn target(&self) -> &Path {
    &self.target
  }
}

impl Iterator for ResourcePaths<'_> {
  type Item = crate::Result<PathBuf>;

  fn next(&mut self) -> Option<crate::Result<PathBuf>> {
    self
      .iter
      .next()
      .map(|resource| resource.map(|res| res.path))
  }
}

fn normalize(path: &Path) -> PathBuf {
  let mut dest = PathBuf::new();
  for component in path.components() {
    match component {
      Component::Prefix(_) => {}
      Component::RootDir => dest.push("/"),
      Component::CurDir => {}
      Component::ParentDir => dest.push(".."),
      Component::Normal(string) => dest.push(string),
    }
  }
  dest
}

impl Iterator for ResourcePathsIter<'_> {
  type Item = crate::Result<Resource>;

  fn next(&mut self) -> Option<crate::Result<Resource>> {
    loop {
      if let Some(ref mut walk_entries) = self.walk_iter {
        if let Some(entry) = walk_entries.next() {
          let entry = match entry {
            Ok(entry) => entry,
            Err(error) => return Some(Err(crate::Error::from(error))),
          };
          if entry.file_type().is_file() {
            let path = entry.into_path();
            let root = self
              .current_pattern
              .as_ref()
              .expect("current pattern should exist while walking");
            let rel_path = path
              .strip_prefix(root)
              .expect("walked path should be inside the root path");
            let target = self
              .current_dest
              .as_ref()
              .map(|dest| dest.join(rel_path))
              .unwrap_or_else(|| resource_relpath(&path));
            return Some(Ok(Resource { path, target }));
          }
        } else {
          self.walk_iter = None;
        }
      }

      if let Some(ref mut glob_entries) = self.glob_iter {
        if let Some(entry) = glob_entries.next() {
          match entry {
            Ok(path) if path.is_file() => {
              let target = self
                .current_dest
                .as_ref()
                .cloned()
                .unwrap_or_else(|| resource_relpath(&path));
              return Some(Ok(Resource { path, target }));
            }
            Ok(_) => {}
            Err(error) => return Some(Err(crate::Error::from(error))),
          }
        } else {
          self.glob_iter = None;
        }
      }

      let (pattern, dest) = match &mut self.pattern_iter {
        PatternIter::Slice(iter) => iter.next().map(|pattern| (pattern.clone(), None)),
        PatternIter::Map(iter) => iter
          .next()
          .map(|(pattern, dest)| (pattern.clone(), Some(PathBuf::from(dest)))),
      }?;

      self.current_dest = dest;

      if pattern.contains('*') {
        match glob::glob(&pattern) {
          Ok(glob_iter) => {
            self.glob_iter = Some(glob_iter);
          }
          Err(error) => return Some(Err(crate::Error::from(error))),
        }
        continue;
      }

      let path = normalize(Path::new(&pattern));
      if path.is_dir() {
        if self.allow_walk {
          self.current_pattern = Some(path.clone());
          self.walk_iter = Some(walkdir::WalkDir::new(&path).into_iter());
          continue;
        }
        return Some(Err(crate::Error::GenericError(format!(
          "expected a file but found a directory: {}",
          path.display()
        ))));
      }

      if path.exists() {
        let target = self
          .current_dest
          .as_ref()
          .cloned()
          .unwrap_or_else(|| resource_relpath(&path));
        return Some(Ok(Resource { path, target }));
      }

      return Some(Err(crate::Error::GenericError(format!(
        "resource path does not exist: {}",
        path.display()
      ))));
    }
  }
}
