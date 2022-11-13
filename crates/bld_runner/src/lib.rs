mod platform;
mod sync;

pub use sync::runner::Runner;
pub use sync::pipeline::Pipeline;
pub use sync::builder::RunnerBuilder;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
