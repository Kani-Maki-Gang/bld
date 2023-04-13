use bld_config::definitions::{
    KEYWORD_BLD_DIR, KEYWORD_ENV, KEYWORD_RUN_PROPS_ID, KEYWORD_RUN_PROPS_START_TIME, KEYWORD_VAR,
};

pub trait Keyword<'a> {
    fn token() -> &'a str;
}

pub struct BldDirectory;

impl<'a> Keyword<'a> for BldDirectory {
    fn token() -> &'a str {
        KEYWORD_BLD_DIR
    }
}

pub struct Variable;

impl<'a> Keyword<'a> for Variable {
    fn token() -> &'a str {
        KEYWORD_VAR
    }
}

pub struct Environment;

impl<'a> Keyword<'a> for Environment {
    fn token() -> &'a str {
        KEYWORD_ENV
    }
}

pub struct RunId;

impl<'a> Keyword<'a> for RunId {
    fn token() -> &'a str {
        KEYWORD_RUN_PROPS_ID
    }
}

pub struct RunStartTime;

impl<'a> Keyword<'a> for RunStartTime {
    fn token() -> &'a str {
        KEYWORD_RUN_PROPS_START_TIME
    }
}
