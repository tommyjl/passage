#[macro_export]
macro_rules! default_env {
    ($var:expr, $value:expr) => {
        if ::std::env::var($var).is_err() {
            ::std::env::set_var($var, $value);
        }
    };
}
