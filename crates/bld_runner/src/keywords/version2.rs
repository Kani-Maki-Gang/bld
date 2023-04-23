use bld_config::definitions::{
    KEYWORD_BLD_DIR_V2, KEYWORD_RUN_PROPS_ID_V2, KEYWORD_RUN_PROPS_START_TIME_V2,
};

pub trait Keyword<'a> {
    fn token() -> &'a str;
}

pub struct BldDirectory;

impl<'a> Keyword<'a> for BldDirectory {
    fn token() -> &'a str {
        KEYWORD_BLD_DIR_V2
    }
}

pub struct Variable;

pub struct Environment;

pub struct RunId;

impl<'a> Keyword<'a> for RunId {
    fn token() -> &'a str {
        KEYWORD_RUN_PROPS_ID_V2
    }
}

pub struct RunStartTime;

impl<'a> Keyword<'a> for RunStartTime {
    fn token() -> &'a str {
        KEYWORD_RUN_PROPS_START_TIME_V2
    }
}
