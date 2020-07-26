use parse_mediawiki_dump::Error as DumpParsingError;
use serde_cbor::Error as SerdeCborError;
use serde_json::{self, error::Error as SerdeJsonError};
use std::path::PathBuf;
use std::{fmt::Display, io::Error as IoError};
use template_iter::TitleNormalizationError;

use crate::args::DumpFileError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    IoError {
        action: &'static str,
        path: PathBuf,
        cause: IoError,
    },
    DumpParsingError(DumpParsingError),
    SerdeJsonError(SerdeJsonError),
    SerdeCborError(SerdeCborError),
    NamespaceConversionError(u32),
    TemplateNameNormalization {
        title: String,
        cause: TitleNormalizationError,
    },
    DumpFileError(DumpFileError),
    ParseTemplateNormalization {
        path: PathBuf,
        cause: SerdeJsonError,
    },
    FormatError {
        description: &'static str,
        path: PathBuf,
        line_number: usize,
        line: String,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IoError {
                action,
                path,
                cause,
            } => {
                write!(f, "failed to {} {}: {}", action, path.display(), cause)
            }
            Error::DumpParsingError(e) => {
                write!(f, "error while parsing dump: {}", e)
            }
            Error::SerdeJsonError(e) => {
                write!(f, "error writing or reading JSON: {}", e)
            }
            Error::NamespaceConversionError(namespace) => {
                write!(f, "namespace {} could not be converted", namespace)
            }
            Error::TemplateNameNormalization { title, cause } => write!(
                f,
                "failed to normalize template name {}: {}",
                title,
                match cause {
                    TitleNormalizationError::TooLong => "too long",
                    TitleNormalizationError::IllegalChar => "illegal character",
                    _ => "unknown error",
                },
            ),
            Error::SerdeCborError(e) => write!(f, "error writing CBOR: {}", e),
            Error::DumpFileError(e) => {
                write!(f, "error finding dump file: {}", e)
            }
            Error::ParseTemplateNormalization { path, cause } => write!(
                f,
                "failed to parse template normalization file {}: {}",
                path.display(),
                cause
            ),
            Error::FormatError {
                description,
                path,
                line_number,
                line,
            } => write!(f, "{} in line {} of {}: {}", description, line_number, path.display(), line),
        }
    }
}

macro_rules! impl_from {
    ($into_enum:ident <-
        [
            $(
                $type_and_variant:ident
                $(($extra_member:ident))?
            ),+
            $(,)?
        ]
    ) => {
        $(
            impl From<$type_and_variant> for $into_enum {
                fn from(e: $type_and_variant) -> $into_enum {
                    $into_enum::$type_and_variant(e$(, $extra_member)?)
                }
            }
        )+
    };
}

impl_from! {
    Error <- [DumpFileError, DumpParsingError, SerdeCborError, SerdeJsonError]
}
