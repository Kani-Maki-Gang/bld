use actix_web::web::Data;
use std::sync::Arc;

pub trait IntoArc {
    fn into_arc(self) -> Arc<Self>
    where
        Self: Sized,
    {
        Arc::new(self)
    }
}

impl<T> IntoArc for T {}

pub trait IntoData {
    fn into_data(self) -> Data<Self>
    where
        Self: Sized,
    {
        Data::new(self)
    }
}

impl<T> IntoData for T {}
