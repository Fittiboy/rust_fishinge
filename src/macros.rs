#[macro_export]
macro_rules! if_err_writer {
    ($func_call:expr, $writer:expr, $($action:expr),*) => {
        if let Err(err) = $func_call {
            write_expect!($writer, &err.to_string());
            $($action;)*
            return Err(err.into());
        }
    };
}

#[macro_export]
macro_rules! let_match_writer {
    ($var_name:tt, $func_call:expr, $writer:expr) => {
        let $var_name = match $func_call {
            Ok($var_name) => $var_name,
            Err(err) => {
                write_expect!($writer, &err.to_string());
                return Err(err.into());
            }
        };
    };
}

#[macro_export]
macro_rules! write_expect {
    ($writer:expr, $to_write:expr) => {
        write_output(&$writer, $to_write).expect("should be able to write to window at this point");
    };
}
