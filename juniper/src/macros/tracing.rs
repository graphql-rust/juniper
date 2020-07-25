#[doc(hidden)]
#[macro_export]
macro_rules! __juniper_trace_internal {
    ($trace_type:ident; $($element:expr),*) => {{
        #[cfg(feature = "tracing")]
        tracing::$trace_type!($($element),*);
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __juniper_trace {
    ($($element:expr),*) => {{
        $crate::__juniper_trace_internal!(trace; $($element),*)
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __juniper_trace_debug {
    ($($element:expr),*) => {{
        $crate::__juniper_trace_internal!(debug; $($element),*)
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __juniper_trace_info {
    ($($element:expr),*) => {{
        $crate::__juniper_trace_internal!(info; $($element),*)
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __juniper_trace_warn {
    ($($element:expr),*) => {{
        $crate::__juniper_trace_internal!(warn; $($element),*)
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __juniper_trace_error {
    ($($element:expr),*) => {{
        $crate::__juniper_trace_internal!(error; $($element),*)
    }};
}
