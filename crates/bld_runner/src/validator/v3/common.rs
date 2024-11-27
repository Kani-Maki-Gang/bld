use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    path::PathBuf,
    sync::Arc,
};

use anyhow::{bail, Result};
use bld_config::{
    definitions::{
        KEYWORD_BLD_DIR_V3, KEYWORD_PROJECT_DIR_V3, KEYWORD_RUN_PROPS_ID_V3,
        KEYWORD_RUN_PROPS_START_TIME_V3,
    },
    path, BldConfig,
};
use bld_core::fs::FileSystem;
use regex::Regex;
use tracing::debug;

use crate::inputs::v3::Input;

use super::{ConsumeValidator, Validate, ValidatorContext};

pub fn create_expression_regex() -> Result<Regex> {
    Ok(Regex::new(r"\$\{\{\s*(\b\w+\b)\s*\}\}")?)
}

pub fn create_keywords() -> HashSet<&'static str> {
    let mut keywords = HashSet::new();
    keywords.insert(KEYWORD_BLD_DIR_V3);
    keywords.insert(KEYWORD_PROJECT_DIR_V3);
    keywords.insert(KEYWORD_RUN_PROPS_ID_V3);
    keywords.insert(KEYWORD_RUN_PROPS_START_TIME_V3);
    keywords
}

pub fn create_symbols<'a>(inputs: HashSet<&'a str>, env: HashSet<&'a str>) -> HashSet<&'a str> {
    let mut symbols = HashSet::new();
    symbols.insert(KEYWORD_BLD_DIR_V3);
    symbols.insert(KEYWORD_PROJECT_DIR_V3);
    symbols.insert(KEYWORD_RUN_PROPS_ID_V3);
    symbols.insert(KEYWORD_RUN_PROPS_START_TIME_V3);
    symbols.extend(&inputs);
    symbols.extend(&env);
    symbols
}

pub fn sanitize_symbol<'a>(symbol: &'a str) -> &'a str {
    symbol[3..symbol.len() - 2].trim()
}

pub struct CommonValidator<'a, V: Validate<'a>> {
    validatable: &'a Box<V>,
    config: Arc<BldConfig>,
    fs: Arc<FileSystem>,
    regex: Regex,
    keywords: HashSet<&'a str>,
    symbols: HashSet<&'a str>,
    section: Vec<&'a str>,
    errors: String,
}

impl<'a, V: Validate<'a>> CommonValidator<'a, V> {
    pub fn new(
        validatable: &'a Box<V>,
        config: Arc<BldConfig>,
        fs: Arc<FileSystem>,
        inputs: HashSet<&'a str>,
        env: HashSet<&'a str>,
    ) -> Result<Self> {
        Ok(Self {
            validatable,
            config,
            fs,
            regex: create_expression_regex()?,
            keywords: create_keywords(),
            symbols: create_symbols(inputs, env),
            section: Vec::new(),
            errors: String::new(),
        })
    }
}

impl<'a, V: Validate<'a>> ValidatorContext<'a> for CommonValidator<'a, V> {
    fn get_config(&self) -> Arc<BldConfig> {
        self.config.clone()
    }

    fn get_fs(&self) -> Arc<FileSystem> {
        self.fs.clone()
    }

    fn push_section(&mut self, section: &'a str) {
        self.section.push(section);
    }

    fn pop_section(&mut self) {
        self.section.pop();
    }

    fn clear_section(&mut self) {
        self.section.clear();
    }

    fn append_error(&mut self, error: &str) {
        let section = self.section.join(" > ");
        let _ = writeln!(self.errors, "[{}] {}", section, error);
    }

    fn contains_symbols(&mut self, value: &str) -> bool {
        self.regex.find(value).is_some()
    }

    fn validate_symbols(&mut self, value: &'a str) {
        for symbol in self.regex.find_iter(value).map(|x| x.as_str()) {
            if !self.symbols.contains(sanitize_symbol(symbol)) {
                let section = self.section.join(" > ");
                let _ = writeln!(
                    self.errors,
                    "[{section} > {symbol}] Expression isn't a keyword or variable"
                );
            }
        }
    }

    fn validate_keywords(&mut self, name: &'a str) {
        if self.keywords.contains(name) {
            let section = self.section.join(" > ");
            let _ = writeln!(self.errors, "[{section}] Invalid name, reserved as keyword",);
        }
    }

    fn validate_file_path(&mut self, value: &'a str) {
        if self.contains_symbols(value) {
            return;
        }
        let path = path![value];
        if !path.is_file() {
            let section = self.section.join(" > ");
            let _ = writeln!(self.errors, "[{section} > {value}] File not found");
        }
    }

    fn validate_env(&mut self, env: &'a HashMap<String, String>) {
        for (k, v) in env.iter() {
            debug!("Validating env: {}", k);
            self.section.push(k);
            self.validate_keywords(k);
            self.validate_symbols(v);
            self.section.pop();
        }
    }
}

impl<'a, V: Validate<'a>> ConsumeValidator for CommonValidator<'a, V> {
    async fn validate(mut self) -> Result<()> {
        self.validatable.validate(&mut self).await;

        if self.errors.is_empty() {
            Ok(())
        } else {
            bail!(self.errors)
        }
    }
}
