use anyhow::Result;
use futures::Future;
use std::{collections::HashMap, pin::Pin};

pub type RecursiveFuture = Pin<Box<dyn Future<Output = Result<HashMap<String, String>>>>>;
