#[macro_use]
extern crate diesel;

pub mod context;
pub mod database;
pub mod execution;
pub mod high_avail;
pub mod logger;
pub mod proxies;
pub mod scanner;
pub mod workers;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
