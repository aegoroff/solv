//! Classic text `.sln` format support.

mod convert;

use crate::api::Solution;
use crate::parser;

/// Parses `.sln` content into [`Solution`]
pub(crate) fn parse_str(contents: &str) -> miette::Result<Solution<'_>> {
    let parsed = parser::parse_str(contents)?;
    Ok(convert::to_api(&parsed))
}
