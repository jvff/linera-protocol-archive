#![no_main]

wit_bindgen::generate!("export-simple-function");

export_export_simple_function!(Implementation);

use self::exports::witty_macros::test_modules::simple_function::SimpleFunction;

struct Implementation;

impl SimpleFunction for Implementation {
    fn simple() {}
}
