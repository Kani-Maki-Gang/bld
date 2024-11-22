use std::{collections::HashSet, sync::Arc};

use anyhow::Result;
use bld_config::BldConfig;
use bld_core::fs::FileSystem;
use regex::Regex;

use crate::{action::v3::Action, traits::Validate};

#[allow(dead_code)]
pub struct ActionValidator<'a> {
    action: &'a Action,
    config: Arc<BldConfig>,
    fs: Arc<FileSystem>,
    regex: Regex,
    keywords: HashSet<&'a str>,
    symbols: HashSet<&'a str>,
    errors: String,
}

impl<'a> Validate for ActionValidator<'a> {
    async fn validate(self) -> Result<()> {
        unimplemented!();
    }
}

impl ActionValidator {
    fn new(action: Action, config: Arc<BldConfig>, fs: Arc<FileSystem>) -> Result<Self> {
        let regex = Regex::new(r"\$\{\{\s*(\b\w+\b)\s*\}\}")?;
        let keywords = Self::prepare_keywords();
        let symbols = Self::prepare_symbols(pipeline);
        let errors = String::new();
        Ok(Self {
            pipeline,
            config,
            fs,
            regex,
            keywords,
            symbols,
            errors,
        })
    }
}
