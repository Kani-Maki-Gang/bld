use std::{collections::HashMap, sync::Arc};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bld_core::regex::RegexCache;
use bld_utils::sync::IntoArc;
use regex::Regex;

use crate::{
    keywords::version2::{BldDirectory, Environment, Keyword, RunId, RunStartTime, Variable},
    pipeline::traits::{TokenContext, TokenTransformer, CompleteTokenTransformer},
};

#[derive(Default)]
pub struct PipelineContextBuilder<'a> {
    bld_directory: Option<&'a str>,
    variables: HashMap<String, String>,
    environment: HashMap<String, String>,
    run_id: Option<&'a str>,
    run_start_time: Option<&'a str>,
    regex_cache: Option<Arc<RegexCache>>,
}

impl<'a> PipelineContextBuilder<'a> {
    pub fn bld_directory(mut self, directory: &'a str) -> Self {
        self.bld_directory = Some(directory);
        self
    }

    pub fn add_variables(mut self, variables: &HashMap<String, String>) -> Self {
        for (k, v) in variables.iter() {
            self.variables.insert(k.to_owned(), v.to_owned());
        }
        self
    }

    pub fn add_environment(mut self, environment: &HashMap<String, String>) -> Self {
        for (k, v) in environment.iter() {
            self.environment.insert(k.to_owned(), v.to_owned());
        }
        self
    }

    pub fn run_id(mut self, run_id: &'a str) -> Self {
        self.run_id = Some(run_id);
        self
    }

    pub fn run_start_time(mut self, run_start_time: &'a str) -> Self {
        self.run_start_time = Some(run_start_time);
        self
    }

    pub fn regex_cache(mut self, regex_cache: Arc<RegexCache>) -> Self {
        self.regex_cache = Some(regex_cache);
        self
    }

    pub fn build(self) -> Result<PipelineContext<'a>> {
        let bld_directory = self
            .bld_directory
            .ok_or_else(|| anyhow!("bld directory not provided in pipeline context"))?;

        let run_id = self
            .run_id
            .ok_or_else(|| anyhow!("run id not provided in pipeline context"))?;

        let run_start_time = self
            .run_start_time
            .ok_or_else(|| anyhow!("run start time not provided in pipeline context"))?;

        let regex_cache = self
            .regex_cache
            .ok_or_else(|| anyhow!("regex cache not provided in pipeline context"))?;

        Ok(PipelineContext {
            bld_directory,
            variables: self.variables,
            environment: self.environment,
            run_id,
            run_start_time,
            regex_cache,
        })
    }
}

pub struct PipelineContext<'a> {
    pub bld_directory: &'a str,
    pub variables: HashMap<String, String>,
    pub environment: HashMap<String, String>,
    pub run_id: &'a str,
    pub run_start_time: &'a str,
    regex_cache: Arc<RegexCache>,
}

impl<'a> PipelineContext<'a> {
    fn get_regex_pattern(keyword: &'a str) -> String {
        format!("{}{}{}", "\\$\\{\\{\\s*", keyword, "\\s*\\}\\}")
    }

    async fn cache_new_regex(&self, pattern: String) -> Result<Arc<Regex>> {
        let re = Regex::new(&pattern)?.into_arc();
        self.regex_cache.set(pattern, re.clone()).await?;
        Ok(re)
    }
}

impl<'a> TokenContext<'a, BldDirectory, &'a str> for PipelineContext<'a> {
    fn retrieve(&'a self) -> &'a str {
        self.bld_directory
    }
}

#[async_trait]
impl<'a> TokenTransformer<'a, BldDirectory, &'a str> for PipelineContext<'a> {
    async fn transform(&'a self, text: String) -> Result<String> {
        let pattern = Self::get_regex_pattern(BldDirectory::token());

        let re = match self.regex_cache.get(pattern.clone()).await? {
            Some(v) => v,
            None => self.cache_new_regex(pattern).await?,
        };

        let result = re
            .replace_all(
                &text,
                TokenContext::<'a, BldDirectory, &'a str>::retrieve(self),
            )
            .to_string();

        Ok(result)
    }
}

impl<'a> TokenContext<'a, Variable, &'a HashMap<String, String>> for PipelineContext<'a> {
    fn retrieve(&'a self) -> &HashMap<String, String> {
        &self.variables
    }
}

#[async_trait]
impl<'a> TokenTransformer<'a, Variable, &'a HashMap<String, String>> for PipelineContext<'a> {
    async fn transform(&'a self, mut text: String) -> Result<String> {
        for (k, v) in TokenContext::<'a, Variable, &'a HashMap<String, String>>::retrieve(self) {
            let pattern = Self::get_regex_pattern(&k);
            let re = match self.regex_cache.get(pattern.clone()).await? {
                Some(v) => v,
                None => self.cache_new_regex(pattern).await?,
            };
            text = re.replace_all(&text, v).to_string();
        }
        Ok(text)
    }
}

impl<'a> TokenContext<'a, Environment, &'a HashMap<String, String>> for PipelineContext<'a> {
    fn retrieve(&'a self) -> &HashMap<String, String> {
        &self.environment
    }
}

#[async_trait]
impl<'a> TokenTransformer<'a, Environment, &'a HashMap<String, String>> for PipelineContext<'a> {
    async fn transform(&'a self, mut text: String) -> Result<String> {
        for (k, v) in TokenContext::<'a, Environment, &'a HashMap<String, String>>::retrieve(self) {
            let pattern = Self::get_regex_pattern(&k);
            let re = match self.regex_cache.get(pattern.clone()).await? {
                Some(v) => v,
                None => self.cache_new_regex(pattern).await?,
            };
            text = re.replace_all(&text, v).to_string();
        }
        Ok(text)
    }
}

impl<'a> TokenContext<'a, RunId, &'a str> for PipelineContext<'a> {
    fn retrieve(&'a self) -> &'a str {
        self.run_id
    }
}

#[async_trait]
impl<'a> TokenTransformer<'a, RunId, &'a str> for PipelineContext<'a> {
    async fn transform(&'a self, text: String) -> Result<String> {
        let pattern = Self::get_regex_pattern(RunId::token());
        let re = match self.regex_cache.get(pattern.clone()).await? {
            Some(v) => v,
            None => self.cache_new_regex(pattern).await?,
        };
        Ok(re
            .replace_all(&text, TokenContext::<'a, RunId, &'a str>::retrieve(self))
            .to_string())
    }
}

impl<'a> TokenContext<'a, RunStartTime, &'a str> for PipelineContext<'a> {
    fn retrieve(&'a self) -> &'a str {
        self.run_start_time
    }
}

#[async_trait]
impl<'a> TokenTransformer<'a, RunStartTime, &'a str> for PipelineContext<'a> {
    async fn transform(&'a self, text: String) -> Result<String> {
        let pattern = Self::get_regex_pattern(RunStartTime::token());
        let re = match self.regex_cache.get(pattern.clone()).await? {
            Some(v) => v,
            None => self.cache_new_regex(pattern).await?,
        };
        Ok(re
            .replace_all(
                &text,
                TokenContext::<'a, RunStartTime, &'a str>::retrieve(self),
            )
            .to_string())
    }
}

#[async_trait]
impl<'a> CompleteTokenTransformer<'a> for PipelineContext<'a> {
    async fn transform(&'a self, mut text: String) -> Result<String> {
        text = <Self as TokenTransformer<'a, BldDirectory, &'a str>>::transform(self, text).await?;
        text = <Self as TokenTransformer<'a, Variable, &'a HashMap<String, String>>>::transform(
            self, text,
        )
        .await?;
        text = <Self as TokenTransformer<'a, Environment, &'a HashMap<String, String>>>::transform(
            self, text,
        )
        .await?;
        text = <Self as TokenTransformer<'a, RunId, &'a str>>::transform(self, text).await?;
        <Self as TokenTransformer<'a, RunStartTime, &'a str>>::transform(self, text).await
    }
}
