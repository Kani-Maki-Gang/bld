pub mod errors;
pub mod fs;
pub mod request;
pub mod sync;
pub mod term;
pub mod tls;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
