use actix_web::web::Data;
use std::sync::Arc;

pub trait AsArc {
    fn as_arc(self) -> Arc<Self>
    where
        Self: Sized,
    {
        Arc::new(self)
    }
}

impl<T> AsArc for T {}

pub trait AsData {
    fn as_data(self) -> Data<Self>
    where
        Self: Sized,
    {
        Data::new(self)
    }
}

impl<T> AsData for T {}
