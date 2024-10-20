use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};

/// A pattern describing a set of targets.
#[derive(Debug)]
#[derive(PartialEq)]
pub struct TargetPattern {
  pub package: String,
  pub scope: PatternScope,
}

/// A scope defining which targets in a package to include.
#[derive(Debug)]
#[derive(PartialEq)]
pub enum PatternScope {
  /// References just a single target of a specified name in the given package.
  SingleTarget(String),

  /// References all targets directly within the package.
  Package,

  /// References lal targets directly within the package *and* all targets
  /// within all descendant packages.
  Descendants,
}

impl TargetPattern {
  /// Parses a `//path/to/pkg:target` string into an `Ok(TargetPattern)`.
  /// Returns an `Err(ParseError)` if the input string does not match the
  /// expected format.
  pub fn parse(pattern: &str) -> Result<TargetPattern, ParseError> {
    // Require leading `//`.
    let without_leading_slashes = pattern.strip_prefix("//");
    if let None = without_leading_slashes {
      return Err(ParseError(format!(
        "Failed to parse `{}`, target patterns must start with `//`.",
        pattern,
      )));
    }

    // Parse `pkg:target`.
    let parts: Vec<_> = without_leading_slashes.unwrap().split(":").collect();
    match parts[..] {
      // No `:`, check for `/...`.
      [ pattern ] => {
        match pattern.strip_suffix("/...") {
          Some(package) => Ok(TargetPattern {
            package: package.to_owned(),
            scope: PatternScope::Descendants,
          }),
          None => Err(ParseError(format!(
            "Failed to parse `{}`, target patterns must end with `:target` or `/...`.",
            pattern,
          ))),
        }
      },

      // One `:`, check for `:all` or `:some_target`.
      [ package, target ] => {
        if target == "all" {
          Ok(TargetPattern {
            package: package.to_owned(),
            scope: PatternScope::Package,
          })
        } else {
          Ok(TargetPattern {
            package: package.to_owned(),
            scope: PatternScope::SingleTarget(target.to_owned()),
          })
        }
      },

      // Multiple `:` characters, error.
      _ => Err(ParseError(format!(
        "Failed to parse `{}`, target patterns may contain at most one `:`.",
        pattern,
      ))),
    }
  }
}

impl Display for TargetPattern {
  fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
    match &self.scope {
      PatternScope::SingleTarget(target) => {
        write!(f, "//{}:{}", self.package, target)
      },
      PatternScope::Package => {
        write!(f, "//{}:all", self.package)
      },
      PatternScope::Descendants => {
        write!(f, "//{}/...", self.package)
      },
    }
  }
}

/// An error from parsing an incorrectly formatted `TargetPattern`.
#[derive(Debug)]
#[derive(PartialEq)]
pub struct ParseError(pub String);

impl Display for ParseError {
  fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
    write!(f, "{}", self.0)
  }
}

impl Error for ParseError {
  fn description(&self) -> &str {
    &self.0
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use assertables::assert_contains;

  #[test]
  fn parse_parses_single_target() {
    assert_eq!(TargetPattern::parse("//path/to/pkg:target"), Ok(TargetPattern {
      package: "path/to/pkg".to_owned(),
      scope: PatternScope::SingleTarget("target".to_owned()),
    }))
  }

  #[test]
  fn parse_parses_package_scope() {
    assert_eq!(TargetPattern::parse("//path/to/pkg:all"), Ok(TargetPattern {
      package: "path/to/pkg".to_owned(),
      scope: PatternScope::Package,
    }))
  }

  #[test]
  fn parse_parses_descendants_scope() {
    assert_eq!(TargetPattern::parse("//path/to/pkg/..."), Ok(TargetPattern {
      package: "path/to/pkg".to_owned(),
      scope: PatternScope::Descendants,
    }))
  }

  #[test]
  fn parse_non_absolute_package_errors() {
    let err = TargetPattern::parse("relative/path/to/pkg:target").unwrap_err();

    assert_contains!(err.0, "must start with `//`");
  }

  #[test]
  fn parse_pattern_without_target_errors() {
    let err = TargetPattern::parse("//path/to/pkg").unwrap_err();

    assert_contains!(err.0, "must end with `:target` or `/...`");
  }

  #[test]
  fn parse_pattern_with_multiple_colons_errors() {
    let err = TargetPattern::parse("//path/to/pkg:foo:bar").unwrap_err();

    assert_contains!(err.0, "may contain at most one `:`");
  }

  #[test]
  fn displays_single_target_pattern() {
    assert_eq!(
      format!("{}", TargetPattern {
        package: "path/to/pkg".to_owned(),
        scope: PatternScope::SingleTarget("target".to_owned()),
      }),
      "//path/to/pkg:target",
    );
  }

  #[test]
  fn displays_package_scope_pattern() {
    assert_eq!(
      format!("{}", TargetPattern {
        package: "path/to/pkg".to_owned(),
        scope: PatternScope::Package,
      }),
      "//path/to/pkg:all",
    );
  }

  #[test]
  fn displays_descendant_scope_pattern() {
    assert_eq!(
      format!("{}", TargetPattern {
        package: "path/to/pkg".to_owned(),
        scope: PatternScope::Descendants,
      }),
      "//path/to/pkg/...",
    );
  }
}
