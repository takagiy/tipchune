use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum PrimitiveError {
    #[error("{}", 0)]
    Uncertain(String),
}

pub type Result<T> = std::result::Result<T, PrimitiveError>;

#[macro_export]
macro_rules! err {
    ($($e:expr),+) => {
        crate::primitive::PrimitiveError::Uncertain(format!($($e),+))
    };
}
