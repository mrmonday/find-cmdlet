use std::error::Error;
use std::fmt::Display;

#[derive(Debug)]
pub enum FindCmdletError {
    MissingMetaDataStart,
    MissingMetaDataEnd,
    UnexpectedMetaData,
    MissingCmdletName,
    MissingCmdletUrl,
    MissingModuleName,
    MissingModuleVersion,
    MissingModJson(String, String, String),
    MissingCmdDir(String, String, String),
    MissingHelpDir(String, String, String),
    MissingCommandFileName(String, String, String),
    MissingHelpText(String, String, String, String),
    TantivyError(tantivy::TantivyError),
}

impl Display for FindCmdletError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            FindCmdletError::MissingMetaDataStart => f.write_str("Missing metadata start for file"),
            FindCmdletError::MissingMetaDataEnd => f.write_str("Missing metadata end for file"),
            FindCmdletError::UnexpectedMetaData => f.write_str("Unexpected metadata format"),
            FindCmdletError::MissingCmdletName => f.write_str("Missing cmdlet name in metadata"),
            FindCmdletError::MissingCmdletUrl => f.write_str("Missing cmdlet URL in metadata"),
            FindCmdletError::MissingModuleName => f.write_str("Missing module name in metadata"),
            FindCmdletError::MissingModuleVersion => {
                f.write_str("Missing module version in metadata")
            }
            FindCmdletError::MissingModJson(name, version, file) => f.write_fmt(format_args!(
                "Missing mod.json file for module {} [{}]: {}",
                name, version, file
            )),
            FindCmdletError::MissingCmdDir(name, version, dir) => f.write_fmt(format_args!(
                "Missing command directory for module {} [{}]: {}",
                name, version, dir
            )),
            FindCmdletError::MissingHelpDir(name, version, dir) => f.write_fmt(format_args!(
                "Missing help directory for module {} [{}]: {}",
                name, version, dir
            )),
            FindCmdletError::MissingCommandFileName(name, version, file) => {
                f.write_fmt(format_args!(
                    "Missing file name for command in module {} [{}]: {}",
                    name, version, file
                ))
            }
            FindCmdletError::MissingHelpText(command, name, version, file) => {
                f.write_fmt(format_args!(
                    "Missing help text for command in {} in module {} [{}]: {}",
                    command, name, version, file
                ))
            }
            FindCmdletError::TantivyError(te) => te.fmt(f),
        }
    }
}

impl Error for FindCmdletError {}
