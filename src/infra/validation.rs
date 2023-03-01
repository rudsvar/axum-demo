//! Utilities for validating constraints on types.

use serde::Deserialize;
use validator::{Validate, ValidationErrors};

/// A type that cannot be instatiated without validating the value within.
/// That is, if you have a [`Valid<T>`], `T` is guaranteed to be valid.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Valid<T> {
    value: T,
}

impl<T> Valid<T> {
    /// Constructs a new validated value.
    pub fn new(value: T) -> Result<Valid<T>, ValidationErrors>
    where
        T: Validate,
    {
        value.validate().map(|_| Valid { value })
    }

    /// Returns a reference to the validated value.
    pub fn inner(&self) -> &T {
        &self.value
    }

    /// Returns the validated value.
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T> AsRef<T> for Valid<T> {
    fn as_ref(&self) -> &T {
        &self.value
    }
}

impl<'de, T: Deserialize<'de> + Validate> Deserialize<'de> for Valid<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value: T = T::deserialize(deserializer)?;
        Valid::new(value).map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::Valid;
    use serde::Deserialize;
    use validator::Validate;

    #[derive(Debug, Validate, Deserialize)]
    struct Fields {
        #[validate(length(min = 4, max = 5))]
        four_or_five: String,
        #[validate(email)]
        email: String,
        #[validate(url)]
        url: String,
        #[validate(range(min = 18, max = 20))]
        age: u32,
    }

    #[test]
    pub fn valid_value_succeeds() {
        let data = r#"
            {
                "four_or_five": "1234",
                "email": "foo@bar.baz",
                "url": "http://foo.bar",
                "age": 19
            }
        "#;
        let value = serde_json::from_str::<Valid<Fields>>(data);
        assert!(value.is_ok());
    }

    #[test]
    pub fn invalid_value_fails() {
        let data = r#"
            {
                "four_or_five": "124",
                "email": "foo@bar.baz",
                "url": "http://foo.bar",
                "age": 19
            }
        "#;
        let value = serde_json::from_str::<Valid<Fields>>(data);
        assert!(value.is_err());
    }
}
