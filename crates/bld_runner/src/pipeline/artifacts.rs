#[derive(Debug)]
pub struct Artifacts {
    pub method: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub ignore_errors: bool,
    pub after: Option<String>,
}

impl Artifacts {
    pub fn new(
        method: Option<String>,
        from: Option<String>,
        to: Option<String>,
        after: Option<String>,
        ignore_errors: bool,
    ) -> Self {
        Self {
            method,
            from,
            to,
            ignore_errors,
            after,
        }
    }
}
