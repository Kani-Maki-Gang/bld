pub mod auth;
mod cli;
pub mod config;
pub mod hist;
pub mod init;
pub mod inspect;
pub mod list;
pub mod monit;
pub mod pull;
pub mod push;
pub mod remove;
pub mod run;
pub mod server;
pub mod stop;
pub mod supervisor;
pub mod worker;

pub use cli::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
