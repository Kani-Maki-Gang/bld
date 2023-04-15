use std::{collections::HashMap, sync::Arc};

use crate::{
    keywords::version2::{BldDirectory, Environment, Keyword, RunId, RunStartTime, Variable},
    pipeline::traits::{
        DynamicTokenContext, DynamicTokenTransformer, HolisticTokenTransformer, StaticTokenContext,
        StaticTokenTransformer,
    },
};

#[derive(Default)]
pub struct PipelineContext<'a> {
    pub bld_directory: &'a str,
    pub variables: Arc<HashMap<String, String>>,
    pub environment: Arc<HashMap<String, String>>,
    pub run_id: &'a str,
    pub run_start_time: &'a str,
}

impl<'a> StaticTokenContext<'a, BldDirectory> for PipelineContext<'a> {
    fn retrieve(&'a self) -> &'a str {
        self.bld_directory
    }
}

impl<'a> StaticTokenTransformer<'a, BldDirectory> for PipelineContext<'a> {
    fn transform(&'a self, text: String) -> String {
        text.replace(
            BldDirectory::token(),
            StaticTokenContext::<'a, BldDirectory>::retrieve(self),
        )
    }
}

impl<'a> DynamicTokenContext<'a, Variable> for PipelineContext<'a> {
    fn retrieve(&'a self) -> HashMap<String, String> {
        let variable_token = Variable::token();
        self.variables
            .iter()
            .map(|(k, v)| (format!("{variable_token}{k}"), v.to_owned()))
            .collect()
    }
}

impl<'a> DynamicTokenTransformer<'a, Variable> for PipelineContext<'a> {
    fn transform(&'a self, mut text: String) -> String {
        for (k, v) in DynamicTokenContext::<'a, Variable>::retrieve(self) {
            text = text.replace(&k, &v);
        }
        text
    }
}

impl<'a> DynamicTokenContext<'a, Environment> for PipelineContext<'a> {
    fn retrieve(&'a self) -> HashMap<String, String> {
        let variable_token = Variable::token();
        self.environment
            .iter()
            .map(|(k, v)| (format!("{variable_token}{k}"), v.to_owned()))
            .collect()
    }
}

impl<'a> DynamicTokenTransformer<'a, Environment> for PipelineContext<'a> {
    fn transform(&'a self, mut text: String) -> String {
        for (k, v) in DynamicTokenContext::<'a, Environment>::retrieve(self) {
            text = text.replace(&k, &v);
        }
        text
    }
}

impl<'a> StaticTokenContext<'a, RunId> for PipelineContext<'a> {
    fn retrieve(&'a self) -> &'a str {
        self.run_id
    }
}

impl<'a> StaticTokenTransformer<'a, RunId> for PipelineContext<'a> {
    fn transform(&'a self, text: String) -> String {
        text.replace(
            RunId::token(),
            StaticTokenContext::<'a, RunId>::retrieve(self),
        )
    }
}

impl<'a> StaticTokenContext<'a, RunStartTime> for PipelineContext<'a> {
    fn retrieve(&'a self) -> &'a str {
        self.run_start_time
    }
}

impl<'a> StaticTokenTransformer<'a, RunStartTime> for PipelineContext<'a> {
    fn transform(&'a self, text: String) -> String {
        text.replace(
            RunStartTime::token(),
            StaticTokenContext::<'a, RunStartTime>::retrieve(self),
        )
    }
}

impl<'a> HolisticTokenTransformer<'a> for PipelineContext<'a> {
    fn transform(&'a self, mut text: String) -> String {
        text = <PipelineContext as StaticTokenTransformer<'a, BldDirectory>>::transform(self, text);
        text = <PipelineContext as DynamicTokenTransformer<'a, Variable>>::transform(self, text);
        text = <PipelineContext as DynamicTokenTransformer<'a, Environment>>::transform(self, text);
        text = <PipelineContext as StaticTokenTransformer<'a, RunId>>::transform(self, text);
        text = <PipelineContext as StaticTokenTransformer<'a, RunStartTime>>::transform(self, text);
        text
    }
}
