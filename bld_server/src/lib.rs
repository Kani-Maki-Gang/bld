#![allow(unused_imports)]

pub mod endpoints;
pub mod extractors;
pub mod requests;
pub mod responses;
pub mod sockets;
pub mod state;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
