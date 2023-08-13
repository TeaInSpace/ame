use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{error::AmeError, Result};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum ImagePullPolicy {
    Always,
    Never,
    IfNotPresent,
    None,
}

impl Default for ImagePullPolicy {
    fn default() -> Self {
        Self::None
    }
}

impl From<ImagePullPolicy> for String {
    fn from(value: ImagePullPolicy) -> Self {
        match value {
            ImagePullPolicy::Always => "Always",
            ImagePullPolicy::Never => "Never",
            ImagePullPolicy::None => "",
            ImagePullPolicy::IfNotPresent => "IfNotPresent",
        }
        .to_string()
    }
}

impl TryFrom<&str> for ImagePullPolicy {
    type Error = AmeError;

    fn try_from(value: &str) -> Result<Self> {
        if &String::from(ImagePullPolicy::Always) == value {
            return Ok(ImagePullPolicy::Always);
        }

        if &String::from(ImagePullPolicy::Never) == value {
            return Ok(ImagePullPolicy::Never);
        }

        if &String::from(ImagePullPolicy::IfNotPresent) == value {
            return Ok(ImagePullPolicy::IfNotPresent);
        }

        if &String::from(ImagePullPolicy::None) == value {
            return Ok(ImagePullPolicy::None);
        }

        Err(AmeError::Parsing(format!(
            "failed to parse image pull policy: {value}, expected one of Always, Never, IfNotPresent or an empty string "
        )))
    }
}

impl TryFrom<String> for ImagePullPolicy {
    type Error = AmeError;

    fn try_from(value: String) -> Result<Self> {
        ImagePullPolicy::try_from(value.as_str())
    }
}

impl FromStr for ImagePullPolicy {
    type Err = AmeError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Self::try_from(s)
    }
}
