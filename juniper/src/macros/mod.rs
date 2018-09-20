// Wrapper macros which allows built-in macros to be recognized as "crate-local".

#[doc(hidden)]
#[macro_export]
macro_rules! __graphql__panic {
    ($($t:tt)*) => ( panic!($($t)*) );
}

#[doc(hidden)]
#[macro_export]
macro_rules! __graphql__stringify {
    ($($t:tt)*) => ( stringify!($($t)*) );
}

#[doc(hidden)]
#[macro_export]
macro_rules! __graphql__concat {
    ($($t:tt)*) => ( concat!($($t)*) );
}

#[doc(hidden)]
#[macro_export]
macro_rules! __graphql__compile_error {
    ($t:expr) => ( compile_error!($t) );
}

#[macro_use]
mod common;
#[macro_use]
mod object;
#[macro_use]
mod interface;
#[macro_use]
mod scalar;
#[macro_use]
mod union;

#[cfg(test)]
mod tests;
