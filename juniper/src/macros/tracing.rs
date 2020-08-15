// Macros to instrument future spans.

#[doc(hidden)]
#[macro_export]
macro_rules! __juniper_instrument_internal {
    ($trace_type:ident; $fut:expr, $($element:expr),*) => {{
        #[cfg(feature = "tracing")]
        {
            $crate::tracing::Instrument::instrument($fut, tracing::$trace_type!($($element),*))
        }
        #[cfg(not(feature = "tracing"))]
        {
            $fut
        }
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __juniper_instrument_trace {
    ($fut:expr, $($element:expr),*) => {{
        $crate::__juniper_instrument_internal!(trace_span; $fut, $($element),*)
    }}
}

// Macros to instrument (non-future) spans.

#[doc(hidden)]
#[macro_export]
macro_rules! __juniper_span_internal {
    ($trace_type:ident; $($element:expr),*) => {
        #[cfg(feature = "tracing")]
        let myspan = $crate::tracing::span!($crate::tracing::Level::$trace_type, ($($element),*));
        #[cfg(feature = "tracing")]
        let _enter = myspan.enter();
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __juniper_span_trace {
    ($($element:expr),*) => {
        $crate::__juniper_span_internal!(TRACE; $($element),*);
    }
}

// Macros to instrument events.

#[doc(hidden)]
#[macro_export]
macro_rules! __juniper_trace_internal {
    ($trace_type:ident; $($element:expr),*) => {{
        #[cfg(feature = "tracing")]
        {
            $crate::tracing::$trace_type!($($element),*);
        }
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
