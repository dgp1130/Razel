use std::{error::Error, fs, path::{self, Path, PathBuf}};
use super::host::{Entry, EntryKind, ExternalPathError, Host};

/// A `Host` implementation which reads off the file system.
pub struct FsHost {
  wksp_root: PathBuf,
}

impl FsHost {
  /// Returns an `FsHost` using the given path as the workspace root.
  fn from(wksp_root: &Path) -> Result<FsHost, Box<dyn Error>> {
    Ok(FsHost {
      wksp_root: wksp_root.canonicalize()?,
    })
  }
}

impl Host for FsHost {
  fn read_to_string(&self, path: &Path) -> Result<String, Box<dyn Error>> {
    let resolved = normalize(&self.wksp_root.join(path))?;

    if !&resolved.starts_with(&self.wksp_root) {
      return Err(Box::new(ExternalPathError(
        format!("Path \"{}\" is outside the workspace.",
        &path.to_str().unwrap()),
      )));
    }

    Ok(fs::read_to_string(resolved)?)
  }

  fn list(&self, path: &Path) -> Result<Vec<Entry>, Box<dyn Error>> {
    let resolved = normalize(&self.wksp_root.join(&path))?;

    if !&resolved.starts_with(&self.wksp_root) {
      return Err(Box::new(ExternalPathError(
        format!("Path \"{}\" is outside the workspace.",
        &path.to_str().unwrap()),
      )));
    }

    Ok(fs::read_dir(&resolved)?
      .map(|entry_result| entry_result
        .and_then(|dir_entry| Ok(Entry {
          path: path.join(dir_entry.file_name()),
          kind: if dir_entry.file_type()?.is_file() {
            EntryKind::File
          } else {
            EntryKind::Directory
          },
        }))
      ).collect::<Vec<Result<_, _>>>()
      .into_iter()
      .collect::<Result<Vec<_>, _>>()?)
  }
}

fn normalize(path: &Path) -> Result<PathBuf, Box<dyn Error>> {
  let p = path::absolute(&path)?;
  let mut stack = Vec::new();
  for component in p.components() {
    let component = component.as_os_str().to_str().unwrap();
    if component == "." {
      continue;
    } else if component == ".." {
      stack.pop();
    } else {
      stack.push(component);
    }
  }

  Ok(PathBuf::from_iter(stack))
}

#[cfg(test)]
mod test {
  use super::*;
  use assertables::{assert_err, assert_set_eq, assert_set_impl_prep};
  use rand::random;
  use std::{env::temp_dir, io, path::PathBuf};

  struct TestDir {
    root: PathBuf,
  }

  impl TestDir {
    fn from<const SIZE: usize>(files: [(&Path, &str); SIZE]) ->
        Result<TestDir, io::Error> {
      let root = temp_dir().join(format!(
        "razel-fshost-{:0>5}", // Left-pad random number.
        random::<i16>(),
      ));
      fs::create_dir_all(&root)?;

      for (path, contents) in files.iter() {
        let resolved = root.join(path);
        let dir = resolved.parent().unwrap();
        fs::create_dir_all(&dir)?;

        fs::write(resolved, contents)?;
      }

      Ok(TestDir { root })
    }
  }

  impl Drop for TestDir {
    fn drop(&mut self) {
      fs::remove_dir_all(&self.root).unwrap();
    }
  }

  #[test]
  fn read_to_string_reads_file() -> Result<(), Box<dyn Error>> {
    let dir = TestDir::from([
      (Path::new("foo.txt"), "Hello, World!"),
    ])?;

    let host = FsHost::from(&dir.root)?;

    assert_eq!(host.read_to_string(Path::new("foo.txt"))?, "Hello, World!");

    Ok(())
  }

  #[test]
  fn read_to_string_reads_nested_file() -> Result<(), Box<dyn Error>> {
    let dir = TestDir::from([
      (Path::new("foo/bar/baz.txt"), "Hello, World!"),
    ])?;

    let host = FsHost::from(&dir.root)?;

    assert_eq!(
      host.read_to_string(Path::new("foo/bar/baz.txt"))?,
      "Hello, World!",
    );

    Ok(())
  }

  #[test]
  fn read_to_string_errors_on_missing_file() -> Result<(), Box<dyn Error>> {
    let dir = TestDir::from([])?;

    let host = FsHost::from(&dir.root)?;

    assert_err!(host.read_to_string(Path::new("foo.txt")));

    Ok(())
  }

  #[test]
  fn read_to_string_errors_on_external_directory() -> Result<(), Box<dyn Error>> {
    let dir = TestDir::from([])?;

    let host = FsHost::from(&dir.root)?;

    assert_err!(host.read_to_string(Path::new("/foo")));
    assert_err!(host.read_to_string(Path::new("foo/../../../bar")));

    Ok(())
  }

  #[test]
  fn list_finds_files_in_directory() -> Result<(), Box<dyn Error>> {
    let dir = TestDir::from([
      (Path::new("foo.txt"), ""),
      (Path::new("bar.txt"), ""),
      (Path::new("baz/test.txt"), ""),
    ])?;

    let host = FsHost::from(&dir.root)?;

    assert_set_eq!(
      host.list(Path::new(""))?,
      [
        Entry {
          path: PathBuf::from("foo.txt"),
          kind: EntryKind::File,
        },
        Entry {
          path: PathBuf::from("bar.txt"),
          kind: EntryKind::File,
        },
        Entry {
          path: PathBuf::from("baz"),
          kind: EntryKind::Directory,
        },
      ],
    );

    Ok(())
  }

  #[test]
  fn list_finds_files_in_subdirectory() -> Result<(), Box<dyn Error>> {
    let dir = TestDir::from([
      (Path::new("dir/foo.txt"), ""),
      (Path::new("dir/bar.txt"), ""),
      (Path::new("dir/baz/test.txt"), ""),
    ])?;

    let host = FsHost::from(&dir.root)?;

    assert_set_eq!(
      host.list(Path::new("dir"))?,
      [
        Entry {
          path: PathBuf::from("dir/foo.txt"),
          kind: EntryKind::File,
        },
        Entry {
          path: PathBuf::from("dir/bar.txt"),
          kind: EntryKind::File,
        },
        Entry {
          path: PathBuf::from("dir/baz"),
          kind: EntryKind::Directory,
        },
      ],
    );

    Ok(())
  }

  #[test]
  fn list_errors_on_missing_directory() -> Result<(), Box<dyn Error>> {
    let dir = TestDir::from([])?;

    let host = FsHost::from(&dir.root)?;

    assert_err!(host.list(Path::new("does/not/exist")));

    Ok(())
  }

  #[test]
  fn list_errors_on_external_directory() -> Result<(), Box<dyn Error>> {
    let dir = TestDir::from([])?;

    let host = FsHost::from(&dir.root)?;

    assert_err!(host.list(Path::new("/foo")));
    assert_err!(host.list(Path::new("foo/../../../bar")));

    Ok(())
  }
}
