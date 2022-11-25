use crate::pipeline::traits::{Load, Parse};
use anyhow::{bail, Result};
use std::collections::HashMap;
use yaml_rust::{Yaml, YamlLoader};

pub enum SyntaxTreeNode {
    Text(String),
    Number(i64),
    Bool(bool),
    Vec(Vec<SyntaxTreeNode>),
    HashMap(HashMap<String, SyntaxTreeNode>),
}

impl Parse<SyntaxTreeNode> for Yaml {
    fn parse(node: &Yaml) -> Result<SyntaxTreeNode> {
        let node = match node {
            Self::Real(value) | Self::String(value) => SyntaxTreeNode::Text(value.to_owned()),

            Self::Integer(value) => SyntaxTreeNode::Number(value.to_owned()),

            Self::Boolean(value) => SyntaxTreeNode::Bool(value.to_owned()),

            Self::Array(value) => {
                let result: Result<Vec<SyntaxTreeNode>> =
                    value.iter().map(|v| Self::parse(v)).collect();
                SyntaxTreeNode::Vec(result?)
            }

            Self::Hash(value) => {
                let result: Result<Vec<(String, SyntaxTreeNode)>> = value
                    .iter()
                    .map(|(k, v)| match k {
                        Self::String(k) => Self::parse(v).map(|r| (k.to_owned(), r)),
                        _ => bail!("yaml hash key can only be a string"),
                    })
                    .collect();
                SyntaxTreeNode::HashMap(result?.into_iter().collect())
            }

            Self::Alias(_) | Self::Null | Self::BadValue => {
                bail!("unsupported token encountered in yaml")
            }
        };

        Ok(node)
    }
}

impl Load<SyntaxTreeNode> for Yaml {
    fn load(input: &str) -> Result<SyntaxTreeNode> {
        let yaml = YamlLoader::load_from_str(input)?;
        if yaml.is_empty() {
            bail!("input is empty");
        }

        let root = match yaml[0] {
            Self::Hash(_) => Self::parse(&yaml[0]),
            _ => bail!("invalid yaml"),
        }?;

        match root {
            SyntaxTreeNode::HashMap(_) => Ok(root),
            _ => bail!("the root of the yaml file should be a hash"),
        }
    }
}
