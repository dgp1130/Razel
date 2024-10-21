use rand::random;
use std::{collections::HashSet, env::temp_dir, error::Error, fmt::Display, fs, path::{Path, PathBuf}};

/// A temporary directory to contain arbitrary test content which is
/// automatically cleaned up when dropped.
#[derive(Debug)]
pub struct TestDir {
  /// The test directory root.
  pub root: PathBuf,
}

impl TestDir {
  /// Creates a test directory in a randomly generated root with the provided
  /// files.
  pub fn from<const SIZE: usize>(files: [(&Path, TestContents); SIZE]) ->
      Result<TestDir, Box<dyn Error>> {
    let root = temp_dir().join(format!(
      "razel-testdir-{:0>5}", // Left-pad random number.
      random::<i16>(),
    ));
    fs::create_dir_all(&root)?;

    let mut written_files = HashSet::new();
    for (path, contents) in files.iter() {
      // Check for duplicate files.
      if written_files.contains(path) {
        return Err(Box::new(DuplicateFileError(
          format!("File `{}` was provided twice.", path.to_str().unwrap()),
        )));
      }

      let resolved = root.join(path);
      match contents {
        // Write file.
        TestContents::File(contents) => {
          let dir = resolved.parent().unwrap();
          fs::create_dir_all(&dir)?;

          fs::write(resolved, contents)?;
          written_files.insert(path);
        },

        // Create directory.
        TestContents::Directory => {
          fs::create_dir_all(resolved)?;
        },
      }
    }

    Ok(TestDir { root })
  }
}

impl Drop for TestDir {
  fn drop(&mut self) {
    // Delete the directory when dropped.
    fs::remove_dir_all(&self.root).unwrap();
  }
}

/// A test directory can directly create files with specific content or empty
/// directories.
pub enum TestContents<'a> {
  /// A file containing the specified content.
  File(&'a str),

  /// An empty directory.
  Directory,
}

#[derive(Debug)]
pub struct DuplicateFileError(String);

impl Error for DuplicateFileError {
  fn description(&self) -> &str {
    &self.0
  }
}

impl Display for DuplicateFileError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", &self.0)
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use assertables::{assert_contains, assert_is_empty};
  use std::error::Error;

  #[test]
  fn test_dir_creates_and_cleans_up_a_temp_directory() -> Result<(), Box<dyn Error>> {
    let test_dir = {
      let dir = TestDir::from([])?;

      assert_eq!(fs::exists(&dir.root)?, true);

      dir.root.clone()
      // `dir` falls out of scope and should delete the test dir.
    };

    assert_eq!(fs::exists(test_dir)?, false);

    Ok(())
  }

  #[test]
  fn test_dir_creates_files() -> Result<(), Box<dyn Error>> {
    let dir = TestDir::from([
      (Path::new("foo.txt"), TestContents::File("foo")),
      (Path::new("bar/baz.txt"), TestContents::File("baz")),
    ])?;

    assert_eq!(fs::read_to_string(&dir.root.join("foo.txt"))?, "foo");
    assert_eq!(fs::read_to_string(&dir.root.join("bar/baz.txt"))?, "baz");

    Ok(())
  }

  #[test]
  fn test_dir_creates_directories() -> Result<(), Box<dyn Error>> {
    let dir = TestDir::from([
      (Path::new("foo"), TestContents::Directory),
      (Path::new("bar/baz"), TestContents::Directory),
    ])?;

    assert_is_empty!(fs::read_dir(&dir.root.join("foo"))?.collect::<Vec<_>>());
    assert_is_empty!(
        fs::read_dir(&dir.root.join("bar/baz"))?.collect::<Vec<_>>());

    Ok(())
  }

  #[test]
  fn test_dir_creates_files_within_existing_directories() -> Result<(), Box<dyn Error>> {
    let dir = TestDir::from([
      (Path::new("foo"), TestContents::Directory),
      (Path::new("foo/bar.txt"), TestContents::File("bar")),
    ])?;

    assert_eq!(fs::read_to_string(dir.root.join("foo/bar.txt"))?, "bar");

    Ok(())
  }

  #[test]
  fn test_dir_ignores_directories_already_created_by_other_files() -> Result<(), Box<dyn Error>> {
    let dir = TestDir::from([
      (Path::new("foo/bar.txt"), TestContents::File("bar")),
      (Path::new("foo"), TestContents::Directory),
    ])?;

    assert_eq!(fs::read_to_string(dir.root.join("foo/bar.txt"))?, "bar");

    Ok(())
  }

  #[test]
  fn test_dir_errors_on_duplicate_files() -> Result<(), Box<dyn Error>> {
    let err = TestDir::from([
      (Path::new("foo.txt"), TestContents::File("bar")),
      (Path::new("foo.txt"), TestContents::File("baz")),
    ]).unwrap_err();

    assert_contains!(err.to_string(), "provided twice");

    Ok(())
  }
}
