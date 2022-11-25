#[derive(Debug)]
pub struct Variable {
    pub name: String,
    pub default_value: String,
}

impl Variable {
    pub fn new(name: String, default_value: String) -> Self {
        Variable {
            name,
            default_value,
        }
    }
}
