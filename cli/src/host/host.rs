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
