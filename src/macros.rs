#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        $crate::kernel::_print(core::format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! println {
    () => {{
        $crate::kernel::_print(core::format_args!("\n"));
    }};

    ($fmt:expr) => {{
        $crate::print!(core::concat!($fmt, "\n"));
    }};

    ($fmt:expr, $($arg:tt)*) => {{
        $crate::print!(core::concat!($fmt, "\n"), $($arg)*);
    }};
}
