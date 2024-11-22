use anyhow::Result;

pub trait ErrorBuilder {
    fn append_error(&mut self, error: &str);
}

pub trait SymbolValidator<'a> {
    fn validate_symbols(&mut self, section: &str, symbol: &'a str);
}

pub trait KeywordValidator<'a> {
    fn validate_keywords(&mut self, section: &str, name: &'a str);
}

#[allow(async_fn_in_trait)]
pub trait ConsumeValidator {
    async fn validate(self) -> Result<()>;
}

pub trait Validatable<'a> {
    fn validate<C>(&self, ctx: &mut C)
    where
        C: ErrorBuilder + SymbolValidator<'a> + KeywordValidator<'a>;
}
