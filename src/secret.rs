#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! secret {
    ($value:expr) => {
        ::obfstr::obfstr!($value)
    };
}

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! secret {
    ($value:expr) => {
        $value
    };
}
