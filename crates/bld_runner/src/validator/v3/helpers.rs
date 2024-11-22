use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
};

use anyhow::Result;
use bld_config::definitions::{
    KEYWORD_BLD_DIR_V3, KEYWORD_PROJECT_DIR_V3, KEYWORD_RUN_PROPS_ID_V3,
    KEYWORD_RUN_PROPS_START_TIME_V3,
};
use regex::Regex;

use crate::traits::Validate;

use super::{ErrorBuilder, KeywordValidator, SymbolValidator, Validatable};

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

pub fn create_symbols<'a>(
    inputs: &'a HashMap<String, String>,
    env: &'a HashMap<String, String>,
) -> HashSet<&'a str> {
    let mut symbols = HashSet::new();
    symbols.insert(KEYWORD_BLD_DIR_V3);
    symbols.insert(KEYWORD_PROJECT_DIR_V3);
    symbols.insert(KEYWORD_RUN_PROPS_ID_V3);
    symbols.insert(KEYWORD_RUN_PROPS_START_TIME_V3);

    for (k, _) in inputs {
        symbols.insert(k);
    }

    for (k, _) in env {
        symbols.insert(k);
    }

    symbols
}

pub fn sanitize_symbol<'a>(symbol: &'a str) -> &'a str {
    symbol[3..symbol.len() - 2].trim()
}

pub struct CommonValidator<'a> {
    regex: Regex,
    keywords: HashSet<&'a str>,
    symbols: HashSet<&'a str>,
    errors: String,
}

impl<'a> CommonValidator<'a> {
    pub fn new(
        inputs: &'a HashMap<String, String>,
        env: &'a HashMap<String, String>,
    ) -> Result<Self> {
        Ok(Self {
            regex: create_expression_regex()?,
            keywords: create_keywords(),
            symbols: create_symbols(inputs, env),
            errors: String::new(),
        })
    }
}

impl<'a> ErrorBuilder for CommonValidator<'a> {
    fn append_error(&mut self, error: &str) {
        let _ = writeln!(self.errors, "{}", error);
    }
}

impl<'a> SymbolValidator<'a> for CommonValidator<'a> {
    fn validate_symbols(&mut self, section: &str, value: &'a str) {
        for symbol in self.regex.find_iter(value).map(|x| x.as_str()) {
            if !self.symbols.contains(sanitize_symbol(symbol)) {
                let _ = writeln!(
                    self.errors,
                    "[{section} > {symbol}] Expression isn't a keyword or variable"
                );
            }
        }
    }
}

impl<'a> KeywordValidator<'a> for CommonValidator<'a> {
    fn validate_keywords(&mut self, section: &str, name: &'a str) {
        if self.keywords.contains(name) {
            let _ = writeln!(self.errors, "[{section}] Invalid name, reserved as keyword",);
        }
    }
}
