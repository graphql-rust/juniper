mod directives;
mod enums;
mod executor;
mod introspection;
mod variables;

// FIXME: re-enable
#[cfg(not(feature = "async"))]
mod interfaces_unions;

#[cfg(feature = "async")]
mod async_await;
