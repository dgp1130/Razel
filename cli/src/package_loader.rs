use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::error::Error;

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
