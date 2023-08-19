#![no_main]

wit_bindgen::generate!("import-simple-function");

export_import_simple_function!(Implementation);

use self::{
    exports::witty_macros::test_modules::entrypoint::Entrypoint,
    witty_macros::test_modules::simple_function::*,
};

struct Implementation;

impl Entrypoint for Implementation {
    fn entrypoint() {
        simple();
    }
}
