#![no_main]

wit_bindgen::generate!("reentrant-simple-function");

export_reentrant_simple_function!(Implementation);

use self::{
    exports::witty_macros::test_modules::{
        entrypoint::Entrypoint, simple_function::SimpleFunction,
    },
    witty_macros::test_modules::simple_function::*,
};

struct Implementation;

impl Entrypoint for Implementation {
    fn entrypoint() {
        simple();
    }
}

impl SimpleFunction for Implementation {
    fn simple() {}
}
