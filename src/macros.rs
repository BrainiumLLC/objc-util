#[macro_export]
macro_rules! os_atleast {
    ($maj:expr $(,)*) => {
        $crate::os_atleast!($maj, 0, 0)
    };
    ($maj:expr, $min:expr $(,)*) => {
        $crate::os_atleast!($maj, $min, 0)
    };
    ($maj:expr, $min:expr, $pat:expr $(,)*) => {
        $crate::OS_VERSION.atleast($maj, $min, $pat)
    };
}

#[macro_export(local_inner_macros)]
macro_rules! sel {
    ($($t:tt)*) => {{
        struct _Dummy;
        impl _Dummy {
            $crate::sel_impl!($($t)*);
        }
        _Dummy::proc_macro_support_wrapper()
    }};
}

#[macro_export(local_inner_macros)]
macro_rules! class {
    ($($t:tt)*) => {{
        struct _Dummy;
        impl _Dummy {
            $crate::class_impl!($($t)*);
        }
        _Dummy::proc_macro_support_wrapper()
    }};
}

#[macro_export(local_inner_macros)]
macro_rules! os_supports {
    ($($t:tt)*) => {{
        struct _Dummy;
        impl _Dummy {
            $crate::os_supports_impl!($($t)*);
        }
        _Dummy::proc_macro_support_wrapper()
    }};
}
