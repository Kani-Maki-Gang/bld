use std::{collections::HashMap, sync::Arc};

use anyhow::{anyhow, Result};
use bld_config::definitions::{
    KEYWORD_BLD_DIR_V3, KEYWORD_PROJECT_DIR_V3, KEYWORD_RUN_PROPS_ID_V3,
    KEYWORD_RUN_PROPS_START_TIME_V3,
};
use bld_core::regex::RegexCache;
use bld_utils::sync::IntoArc;
use regex::Regex;

#[derive(Default)]
pub struct ExecutionContextBuilder<'a> {
    root_dir: Option<&'a str>,
    project_dir: Option<&'a str>,
    inputs: HashMap<String, String>,
    env: HashMap<String, String>,
    run_id: Option<&'a str>,
    run_start_time: Option<&'a str>,
    regex_cache: Option<Arc<RegexCache>>,
}

impl<'a> ExecutionContextBuilder<'a> {
    pub fn root_dir(mut self, directory: &'a str) -> Self {
        self.root_dir = Some(directory);
        self
    }

    pub fn project_dir(mut self, directory: &'a str) -> Self {
        self.project_dir = Some(directory);
        self
    }

    pub fn add_inputs(mut self, inputs: &HashMap<String, String>) -> Self {
        for (k, v) in inputs.iter() {
            self.inputs.insert(k.to_owned(), v.to_owned());
        }
        self
    }

    pub fn add_env(mut self, env: &HashMap<String, String>) -> Self {
        for (k, v) in env.iter() {
            self.env.insert(k.to_owned(), v.to_owned());
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

    pub fn build(self) -> Result<ExecutionContext<'a>> {
        let root_dir = self
            .root_dir
            .ok_or_else(|| anyhow!("bld root directory not provided in pipeline context"))?;

        let project_dir = self
            .project_dir
            .ok_or_else(|| anyhow!("project directory not provided in pipeline context"))?;

        let run_id = self
            .run_id
            .ok_or_else(|| anyhow!("run id not provided in pipeline context"))?;

        let run_start_time = self
            .run_start_time
            .ok_or_else(|| anyhow!("run start time not provided in pipeline context"))?;

        let regex_cache = self
            .regex_cache
            .ok_or_else(|| anyhow!("regex cache not provided in pipeline context"))?;

        Ok(ExecutionContext {
            root_dir,
            project_dir,
            inputs: self.inputs,
            env: self.env,
            run_id,
            run_start_time,
            regex_cache,
        })
    }
}

pub struct ExecutionContext<'a> {
    pub root_dir: &'a str,
    pub project_dir: &'a str,
    pub inputs: HashMap<String, String>,
    pub env: HashMap<String, String>,
    pub run_id: &'a str,
    pub run_start_time: &'a str,
    regex_cache: Arc<RegexCache>,
}

impl<'a> ExecutionContext<'a> {
    fn get_regex_pattern(keyword: &'a str) -> String {
        format!("{}{}{}", r"\$\{\{\s*", keyword, r"\s*\}\}")
    }

    async fn cache_new_regex(&self, pattern: String) -> Result<Arc<Regex>> {
        let re = Regex::new(&pattern)?.into_arc();
        self.regex_cache.set(pattern, re.clone()).await?;
        Ok(re)
    }

    async fn root_dir_transform(&'a self, text: String) -> Result<String> {
        let pattern = Self::get_regex_pattern(KEYWORD_BLD_DIR_V3);

        let re = match self.regex_cache.get(pattern.clone()).await? {
            Some(v) => v,
            None => self.cache_new_regex(pattern).await?,
        };

        let result = re.replace_all(&text, self.root_dir).to_string();

        Ok(result)
    }

    async fn project_dir_transform(&self, text: String) -> Result<String> {
        let pattern = Self::get_regex_pattern(KEYWORD_PROJECT_DIR_V3);

        let re = match self.regex_cache.get(pattern.clone()).await? {
            Some(v) => v,
            None => self.cache_new_regex(pattern).await?,
        };

        let result = re.replace_all(&text, self.project_dir).to_string();

        Ok(result)
    }

    async fn inputs_transform(&'a self, mut text: String) -> Result<String> {
        for (k, v) in self.inputs.iter() {
            let pattern = Self::get_regex_pattern(k);
            let re = match self.regex_cache.get(pattern.clone()).await? {
                Some(v) => v,
                None => self.cache_new_regex(pattern).await?,
            };
            text = re.replace_all(&text, v).to_string();
        }
        Ok(text)
    }

    async fn env_transform(&'a self, mut text: String) -> Result<String> {
        for (k, v) in self.env.iter() {
            let pattern = Self::get_regex_pattern(k);
            let re = match self.regex_cache.get(pattern.clone()).await? {
                Some(v) => v,
                None => self.cache_new_regex(pattern).await?,
            };
            text = re.replace_all(&text, v).to_string();
        }
        Ok(text)
    }

    async fn run_id_transform(&'a self, text: String) -> Result<String> {
        let pattern = Self::get_regex_pattern(KEYWORD_RUN_PROPS_ID_V3);
        let re = match self.regex_cache.get(pattern.clone()).await? {
            Some(v) => v,
            None => self.cache_new_regex(pattern).await?,
        };
        Ok(re.replace_all(&text, self.run_id).to_string())
    }

    async fn run_start_time_transform(&'a self, text: String) -> Result<String> {
        let pattern = Self::get_regex_pattern(KEYWORD_RUN_PROPS_START_TIME_V3);
        let re = match self.regex_cache.get(pattern.clone()).await? {
            Some(v) => v,
            None => self.cache_new_regex(pattern).await?,
        };
        Ok(re.replace_all(&text, self.run_start_time).to_string())
    }

    pub async fn transform(&self, mut text: String) -> Result<String> {
        text = self.root_dir_transform(text).await?;
        text = self.project_dir_transform(text).await?;
        text = self.run_id_transform(text).await?;
        text = self.run_start_time_transform(text).await?;
        text = self.inputs_transform(text).await?;
        self.env_transform(text).await
    }
}
