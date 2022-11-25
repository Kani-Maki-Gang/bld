use crate::pipeline::variable::Variable;

#[derive(Debug)]
pub struct ExternalDetails {
    pub name: String,
    pub pipeline: String,
    pub variables: Vec<Variable>,
    pub environment: Vec<Variable>,
}

impl ExternalDetails {
    pub fn new(
        name: String,
        pipeline: String,
        variables: Vec<Variable>,
        environment: Vec<Variable>,
    ) -> Self {
        Self {
            name,
            pipeline,
            variables,
            environment,
        }
    }
}

#[derive(Debug)]
pub enum External {
    Local(ExternalDetails),
    Server {
        server: String,
        details: ExternalDetails,
    },
}
