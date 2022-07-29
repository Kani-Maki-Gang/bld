pub mod endpoints;
pub mod extractors;
pub mod requests;
pub mod responses;
mod server;
pub mod sockets;

pub use server::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
