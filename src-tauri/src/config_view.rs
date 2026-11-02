//! Configuration problems and save results, as sent to the panel.

use serde::Serialize;
use tunnelbar_core::store::ConfigError;
use tunnelbar_core::validation::ValidationError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConfigProblem {
    /// The file cannot be read or written.
    Io { message: String },
    /// The file is not valid TOML or does not match the format.
    Parse {
        message: String,
        line: Option<usize>,
    },
    /// The file breaks validation rules.
    Invalid { errors: Vec<ValidationError> },
    /// The file on disk is broken: the panel must not overwrite it.
    FileBroken,
}

impl From<&ConfigError> for ConfigProblem {
    fn from(error: &ConfigError) -> Self {
        match error {
            ConfigError::Io { .. } => ConfigProblem::Io {
                message: error.to_string(),
            },
            ConfigError::Parse(parse) => ConfigProblem::Parse {
                message: parse.message.clone(),
                line: parse.line,
            },
            ConfigError::Invalid(errors) => ConfigProblem::Invalid {
                errors: errors.clone(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;
    use std::path::PathBuf;
    use tunnelbar_core::model::ParseError;
    use tunnelbar_core::validation::ValidationErrorKind;

    #[test]
    fn converts_every_config_error() {
        let io = ConfigError::Io {
            path: PathBuf::from("/x"),
            kind: ErrorKind::PermissionDenied,
            message: "denied".into(),
        };
        assert_eq!(
            ConfigProblem::from(&io),
            ConfigProblem::Io {
                message: "cannot access /x: denied".into()
            }
        );

        let parse = ConfigError::Parse(ParseError {
            message: "expected `]`".into(),
            line: Some(3),
        });
        assert_eq!(
            ConfigProblem::from(&parse),
            ConfigProblem::Parse {
                message: "expected `]`".into(),
                line: Some(3)
            }
        );

        let errors = vec![ValidationError {
            tunnel: Some(0),
            forward: None,
            kind: ValidationErrorKind::NoForward,
        }];
        assert_eq!(
            ConfigProblem::from(&ConfigError::Invalid(errors.clone())),
            ConfigProblem::Invalid { errors }
        );
    }

    #[test]
    fn serializes_with_a_kind_tag() {
        let json = serde_json::to_value(ConfigProblem::Parse {
            message: "bad".into(),
            line: Some(2),
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "kind": "parse", "message": "bad", "line": 2 })
        );
    }
}
