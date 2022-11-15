mod platform;
mod sync;

pub use sync::builder::RunnerBuilder;
pub use sync::pipeline::Pipeline;
pub use sync::runner::Runner;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
