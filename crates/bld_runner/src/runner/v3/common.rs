use anyhow::Result;
use futures::Future;
use std::pin::Pin;

pub type RecursiveFuture = Pin<Box<dyn Future<Output = Result<()>>>>;
