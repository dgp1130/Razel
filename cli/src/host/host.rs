use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};

/// The host environment of the build system which allows Razel to interact with
/// the outside world.
pub trait Host {
  /// Reads a file at the given path and returns it as a string. The path is
  /// resolved relative to the workspace root.
  fn read_to_string(&self, path: &Path) -> Result<String, Box<dyn Error>>;

  /// Lists the directory at the given path and returns its entries. The path is
  /// resolved relative to the workspace root.
  fn list(&self, path: &Path) -> Result<Vec<Entry>, Box<dyn Error>>;
}

/// List all recursive files in the given directory. Directories are *not*
/// returned.
pub fn list_all_files(host: &dyn Host, path: &Path) ->
    Result<Vec<PathBuf>, Box<dyn Error>> {
  Ok(
    host.list(path)?.into_iter().map(|entry| match entry.kind {
      EntryKind::File => Ok(vec![entry.path.to_path_buf()]),
      EntryKind::Directory => list_all_files(host, &entry.path)
    }).collect::<Vec<Result<Vec<PathBuf>, Box<dyn Error>>>>()
      .into_iter()
      .collect::<Result<Vec<Vec<PathBuf>>, Box<dyn Error>>>()?
      .into_iter()
      .flatten()
      .collect()
  )
}

/// A file entry.
#[derive(Debug, Eq, Ord, PartialOrd, PartialEq)]
pub struct Entry {
  /// The workspace-relative path of the file.
  pub path: PathBuf,

  /// Whether this is a file or directory.
  pub kind: EntryKind,
}

#[derive(Debug, Eq, Ord, PartialOrd, PartialEq)]
pub enum EntryKind {
  File = 1,
  Directory = 2,
}

/// An error thrown when a file external to the current workspace is requested.
#[derive(Debug)]
pub struct ExternalPathError(pub String);

impl Display for ExternalPathError {
  fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
    write!(f, "{}", &self.0)
  }
}

impl Error for ExternalPathError {
  fn description(&self) -> &str {
    &self.0
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use assertables::{assert_contains, assert_is_empty, assert_set_eq, assert_set_impl_prep};
  use crate::host::fs_host::FsHost;
  use crate::host::test_dir::{TestContents, TestDir};

  #[test]
  fn find_all_files_finds_recursive_files_in_root_directory() -> Result<(), Box<dyn Error>> {
    let dir = TestDir::from([
      (Path::new("foo/bar/baz.txt"), TestContents::File("")),
      (Path::new("foo/bar/hello/world.txt"), TestContents::File("")),
      (Path::new("not/included.txt"), TestContents::File("")),
    ])?;

    let host = FsHost::from(&dir.root)?;

    assert_set_eq!(list_all_files(&host, Path::new(""))?, [
      PathBuf::from("foo/bar/baz.txt"),
      PathBuf::from("foo/bar/hello/world.txt"),
      PathBuf::from("not/included.txt"),
    ]);

    Ok(())
  }

  #[test]
  fn find_all_files_finds_recursive_files_in_subdirectory() -> Result<(), Box<dyn Error>> {
    let dir = TestDir::from([
      (Path::new("foo/bar/baz.txt"), TestContents::File("")),
      (Path::new("foo/bar/hello/world.txt"), TestContents::File("")),
      (Path::new("not/included.txt"), TestContents::File("")),
    ])?;

    let host = FsHost::from(&dir.root)?;

    assert_set_eq!(list_all_files(&host, Path::new("foo"))?, [
      PathBuf::from("foo/bar/baz.txt"),
      PathBuf::from("foo/bar/hello/world.txt"),
    ]);

    Ok(())
  }

  #[test]
  fn find_all_files_empty_directory_returns_no_files() -> Result<(), Box<dyn Error>> {
    let dir = TestDir::from([
      (Path::new("foo"), TestContents::Directory),
      (Path::new("not/included.txt"), TestContents::File("")),
    ])?;

    let host = FsHost::from(&dir.root)?;

    assert_is_empty!(list_all_files(&host, Path::new("foo"))?);

    Ok(())
  }

  #[test]
  fn find_all_files_nonexistent_directory_errors() -> Result<(), Box<dyn Error>> {
    let dir = TestDir::from([])?;

    let host = FsHost::from(&dir.root)?;

    let err = list_all_files(&host, Path::new("foo")).unwrap_err();
    assert_contains!(err.to_string(), "No such file");

    Ok(())
  }
}
