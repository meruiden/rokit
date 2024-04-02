use std::{fmt, str::FromStr};

use semver::Version;
use serde_with::{DeserializeFromStr, SerializeDisplay};
use thiserror::Error;

use crate::sources::ArtifactProvider;

use super::{util::is_invalid_identifier, ToolAlias, ToolSpec};

/**
    Error type representing the possible errors that can occur when parsing a `ToolId`.
*/
#[derive(Debug, Error)]
pub enum ToolIdParseError {
    #[error("tool id is empty")]
    Empty,
    #[error("missing '/' separator")]
    MissingSeparator,
    #[error("artifact provider '{0}' is invalid")]
    InvalidProvider(String),
    #[error("author '{0}' is empty or invalid")]
    InvalidAuthor(String),
    #[error("name '{0}' is empty or invalid")]
    InvalidName(String),
}

/**
    A tool identifier, which includes the author and name of a tool.

    The author and name are case-insensitive, but are stored in both cased
    and uncased forms, meaning [`ToolId::author`] and [`ToolId::name`] will
    always return the original case of the author and name - however,
    comparisons using [`Eq`], [`PartialEq`], [`Ord`], [`PartialOrd`],
    and [`std::hash::Hash`] will always be case-insensitive.

    Also includes the provider of the artifact, which by default is `GitHub`.

    Used to uniquely identify a tool, but not its version.
*/
#[derive(Debug, Clone, DeserializeFromStr, SerializeDisplay)]
pub struct ToolId {
    provider: ArtifactProvider,
    uncased_author: String,
    uncased_name: String,
    cased_author: String,
    cased_name: String,
}

impl ToolId {
    #[must_use]
    pub fn provider(&self) -> ArtifactProvider {
        self.provider
    }

    #[must_use]
    pub fn author(&self) -> &str {
        &self.cased_author
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.cased_name
    }

    #[must_use]
    pub fn into_spec(self, version: Version) -> ToolSpec {
        ToolSpec::from((self, version))
    }

    #[must_use]
    pub fn into_alias(self) -> ToolAlias {
        ToolAlias::from(self)
    }
}

impl Eq for ToolId {}

impl PartialEq for ToolId {
    fn eq(&self, other: &Self) -> bool {
        self.uncased_author == other.uncased_author && self.uncased_name == other.uncased_name
    }
}

impl std::hash::Hash for ToolId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.uncased_author.hash(state);
        self.uncased_name.hash(state);
    }
}

impl Ord for ToolId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.uncased_author
            .cmp(&other.uncased_author)
            .then_with(|| self.uncased_name.cmp(&other.uncased_name))
    }
}

impl PartialOrd for ToolId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl FromStr for ToolId {
    type Err = ToolIdParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(ToolIdParseError::Empty);
        }

        let (provider, after_provider) = match s.split_once(':') {
            None => (ArtifactProvider::default(), s),
            Some((left, right)) => {
                let provider = ArtifactProvider::from_str(left)
                    .map_err(|e| ToolIdParseError::InvalidProvider(e.to_string()))?;
                (provider, right)
            }
        };

        let Some((before, after)) = after_provider.split_once('/') else {
            return Err(ToolIdParseError::MissingSeparator);
        };

        let before = before.trim();
        let after = after.trim();

        if is_invalid_identifier(before) {
            return Err(ToolIdParseError::InvalidAuthor(before.to_string()));
        }
        if is_invalid_identifier(after) {
            return Err(ToolIdParseError::InvalidName(after.to_string()));
        }

        Ok(Self {
            provider,
            uncased_author: before.to_ascii_lowercase(),
            uncased_name: after.to_ascii_lowercase(),
            cased_author: before.to_string(),
            cased_name: after.to_string(),
        })
    }
}

impl fmt::Display for ToolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.cased_author, self.cased_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_id_with_provider(provider: ArtifactProvider, author: &str, name: &str) -> ToolId {
        ToolId {
            provider,
            uncased_author: author.to_ascii_lowercase(),
            uncased_name: name.to_ascii_lowercase(),
            cased_author: author.to_string(),
            cased_name: name.to_string(),
        }
    }

    fn new_id(author: &str, name: &str) -> ToolId {
        new_id_with_provider(ArtifactProvider::default(), author, name)
    }

    #[test]
    fn parse_valid_basic() {
        // Basic strings should parse ok
        assert!("a/b".parse::<ToolId>().is_ok());
        assert!("author/name".parse::<ToolId>().is_ok());
        assert!("123abc456/78de90".parse::<ToolId>().is_ok());
        // The parsed ToolId should match the input
        assert_eq!("a/b".parse::<ToolId>().unwrap(), new_id("a", "b"));
        assert_eq!(
            "author/name".parse::<ToolId>().unwrap(),
            new_id("author", "name")
        );
        assert_eq!(
            "123abc456/78de90".parse::<ToolId>().unwrap(),
            new_id("123abc456", "78de90")
        );
    }

    #[test]
    fn parse_valid_extra_whitespace() {
        // Leading and trailing whitespace should be trimmed and ok
        assert!("a/ b".parse::<ToolId>().is_ok());
        assert!("a/b ".parse::<ToolId>().is_ok());
        assert!("a /b".parse::<ToolId>().is_ok());
        assert!("a/ b".parse::<ToolId>().is_ok());
        assert!("a/b ".parse::<ToolId>().is_ok());
        // The trimmed whitespace should not be in the resulting ToolId
        let id = new_id("a", "b");
        assert_eq!("a/ b".parse::<ToolId>().unwrap(), id);
        assert_eq!("a/b ".parse::<ToolId>().unwrap(), id);
        assert_eq!("a /b".parse::<ToolId>().unwrap(), id);
        assert_eq!("a/ b".parse::<ToolId>().unwrap(), id);
        assert_eq!("a/b ".parse::<ToolId>().unwrap(), id);
    }

    #[test]
    fn parse_valid_provider() {
        // Known provider strings should parse ok
        assert!("github:a/b".parse::<ToolId>().is_ok());
        // The parsed ToolId should match the input
        assert_eq!(
            "github:a/b".parse::<ToolId>().unwrap(),
            new_id_with_provider(ArtifactProvider::GitHub, "a", "b")
        );
    }

    #[test]
    fn parse_invalid_missing() {
        // Empty strings or parts should not be allowed
        assert!("".parse::<ToolId>().is_err());
        assert!("/".parse::<ToolId>().is_err());
        assert!("a/".parse::<ToolId>().is_err());
        assert!("/b".parse::<ToolId>().is_err());
    }

    #[test]
    fn parse_invalid_extra_separator() {
        // Superfluous separators should not be allowed
        assert!("a/b/".parse::<ToolId>().is_err());
        assert!("a/b/c".parse::<ToolId>().is_err());
    }

    #[test]
    fn parse_invalid_provider() {
        // Empty provider should not be allowed
        assert!(":a/b".parse::<ToolId>().is_err());
        assert!(":a/b".parse::<ToolId>().is_err());
        assert!(":a/b".parse::<ToolId>().is_err());
        // Unrecognized provider should not be allowed
        assert!("unknown:a/b".parse::<ToolId>().is_err());
        assert!("hubgit:a/b".parse::<ToolId>().is_err());
        assert!("bitbab:a/b".parse::<ToolId>().is_err());
    }

    #[test]
    fn case_preservation() {
        // The author and name should be preserved in their original case
        assert_eq!(new_id("author", "name").author(), "author");
        assert_eq!(new_id("author", "name").name(), "name");
        assert_eq!(new_id("Author", "Name").author(), "Author");
        assert_eq!(new_id("Author", "Name").name(), "Name");
        assert_eq!(new_id("123abc456", "78de90").author(), "123abc456");
        assert_eq!(new_id("123abc456", "78de90").name(), "78de90");
    }

    #[test]
    fn case_sensitivity_eq() {
        // Case-insensitive comparisons should be equal
        assert_eq!(new_id("a", "b"), new_id("A", "B"));
        assert_eq!(new_id("author", "name"), new_id("Author", "Name"));
        assert_eq!(new_id("123abc456", "78de90"), new_id("123ABC456", "78DE90"));
    }

    #[test]
    fn case_sensitivity_ord() {
        use std::cmp::Ordering;
        // Case-insensitive comparisons should be equal
        assert_eq!(new_id("a", "b").cmp(&new_id("A", "B")), Ordering::Equal);
        assert_eq!(
            new_id("author", "name").cmp(&new_id("Author", "Name")),
            Ordering::Equal
        );
        assert_eq!(
            new_id("123abc456", "78de90").cmp(&new_id("123ABC456", "78DE90")),
            Ordering::Equal
        );
    }

    #[test]
    fn case_sensitivity_hash() {
        use std::collections::HashMap;
        // Case-insensitive comparisons should have the
        // same hash and work in collections such as HashMap
        let mut map = HashMap::new();
        map.insert(new_id("a", "b"), 1);
        map.insert(new_id("author", "name"), 2);
        map.insert(new_id("123abc456", "78de90"), 3);
        assert_eq!(map.get(&new_id("A", "B")), Some(&1));
        assert_eq!(map.get(&new_id("Author", "Name")), Some(&2));
        assert_eq!(map.get(&new_id("123ABC456", "78DE90")), Some(&3));
    }
}
