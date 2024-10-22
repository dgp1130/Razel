use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::{error::Error, fmt::Display};

use crate::host::host::{Host, list_all_files};
use crate::target_pattern::{PatternScope, TargetPattern};

pub struct PackageLoader<'a> {
  host: &'a dyn Host,
}

impl<'a> PackageLoader<'a> {
  pub fn from(host: &'a dyn Host) -> PackageLoader<'a> {
    PackageLoader { host }
  }

  /// Resolves the given `TargetPattern` structs into a list of package paths.
  pub fn resolve_packages(&self, patterns: Vec<TargetPattern>) ->
      Result<HashSet<PathBuf>, Box<dyn Error>> {
    Ok(
      // Get all recursive files in the package.
      patterns.into_iter().map(|pattern| match &pattern.scope {
        PatternScope::SingleTarget(_) | PatternScope::Package =>
          Ok::<HashSet<PathBuf>, Box<dyn Error>>(HashSet::from([
            PathBuf::from(pattern.package).join("BUILD.js"),
          ])),
        PatternScope::Descendants => Ok(list_all_files(
          self.host,
          Path::new(&pattern.package),
        )?.into_iter().collect::<HashSet<PathBuf>>())
      })
        .collect::<Result<Vec<HashSet<PathBuf>>, _>>()?
        .into_iter()
        .flatten()
        // Filter to packages containing `BUILD.js` files
        .filter(|file| file.file_name().unwrap() == "BUILD.js")
        // Get directory containing the `BUILD.js` file.
        .map(|build_file| build_file.parent().unwrap().to_path_buf())
        .collect::<HashSet<PathBuf>>(),
    )
  }

  /// Loads the given package paths by executing them in v8.
  pub fn load_packages(&self, pkgs: &HashSet<&Path>) ->
      Result<(), Box<dyn Error>> {
    let build_pkgs = pkgs.iter()
      // Resolve the `BUILD.js` path.
      .map(|pkg| Path::new(&pkg).join(Path::new("BUILD.js")))
      // Read the `BUILD.js` file.
      .map(|pkg_path| self.host.read_to_string(&pkg_path)
          // Combine package path and build file contents.
          .map(|build_file| (pkg_path, build_file))
      ).collect::<Result<Vec<_>, _>>()?;

    for (pkg_path, build_file) in build_pkgs {
      eprintln!("Loading package: {}", pkg_path.to_str().unwrap());
      eprintln!("Evaluating `{}`:\n{}", pkg_path.to_str().unwrap(), build_file);
    }

    Ok(())
  }
}

#[derive(Debug)]
pub struct LoadError(String);

impl Display for LoadError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", &self.0)
  }
}

impl Error for LoadError {
  fn description(&self) -> &str {
      &self.0
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use assertables::{assert_set_eq, assert_set_impl_prep};
  use crate::host::fs_host::FsHost;
  use crate::host::test_dir::{TestContents, TestDir};
  use crate::target_pattern::TargetPattern;

  #[test]
  fn resolve_packages_resolves_single_target_patterns() -> Result<(), Box<dyn Error>> {
    let dir = TestDir::from([])?;
    let host = FsHost::from(&dir.root)?;

    let loader = PackageLoader::from(&host);

    let pkgs = loader.resolve_packages(vec![
      TargetPattern::parse("//foo:bar")?,
      TargetPattern::parse("//:root")?,
      TargetPattern::parse("//path/to/pkg:target")?,
    ])?;

    assert_set_eq!(pkgs, vec![
      PathBuf::from("foo"),
      PathBuf::from(""),
      PathBuf::from("path/to/pkg"),
    ]);

    Ok(())
  }

  #[test]
  fn resolve_packages_resolves_package_scope_patterns() -> Result<(), Box<dyn Error>> {
    let dir = TestDir::from([])?;
    let host = FsHost::from(&dir.root)?;

    let loader = PackageLoader::from(&host);

    let pkgs = loader.resolve_packages(vec![
      TargetPattern::parse("//foo:all")?,
      TargetPattern::parse("//:all")?,
      TargetPattern::parse("//path/to/pkg:all")?,
    ])?;

    assert_set_eq!(pkgs, vec![
      PathBuf::from("foo"),
      PathBuf::from(""),
      PathBuf::from("path/to/pkg"),
    ]);

    Ok(())
  }

  #[test]
  fn resolve_packages_resolves_descendant_scope_patterns() -> Result<(), Box<dyn Error>> {
    let dir = TestDir::from([
      (Path::new("foo/BUILD.js"), TestContents::File("")),
      (Path::new("foo/bar/baz/BUILD.js"), TestContents::File("")),
      (Path::new("path/to/pkg/BUILD.js"), TestContents::File("")),
      (Path::new("path/to/pkg/deeply/nested/BUILD.js"), TestContents::File("")),
      ])?;
    let host = FsHost::from(&dir.root)?;

    let loader = PackageLoader::from(&host);

    let pkgs = loader.resolve_packages(vec![
      TargetPattern::parse("//foo/...")?,
      TargetPattern::parse("//path/to/pkg/...")?,
    ])?;

    assert_set_eq!(pkgs, vec![
      PathBuf::from("foo"),
      PathBuf::from("foo/bar/baz"),
      PathBuf::from("path/to/pkg"),
      PathBuf::from("path/to/pkg/deeply/nested"),
    ]);

    Ok(())
  }

  #[test]
  fn resolve_packages_resolves_root_package_descendant_pattern() -> Result<(), Box<dyn Error>> {
    let dir = TestDir::from([
      (Path::new("foo/BUILD.js"), TestContents::File("")),
      (Path::new("foo/bar/baz/BUILD.js"), TestContents::File("")),
      (Path::new("path/to/pkg/BUILD.js"), TestContents::File("")),
      (Path::new("path/to/pkg/deeply/nested/BUILD.js"), TestContents::File("")),
      ])?;
    let host = FsHost::from(&dir.root)?;

    let loader = PackageLoader::from(&host);

    let pkgs = loader.resolve_packages(vec![
      TargetPattern::parse("//...")?,
    ])?;

    assert_set_eq!(pkgs, vec![
      PathBuf::from("foo"),
      PathBuf::from("foo/bar/baz"),
      PathBuf::from("path/to/pkg"),
      PathBuf::from("path/to/pkg/deeply/nested"),
    ]);

    Ok(())
  }
}
