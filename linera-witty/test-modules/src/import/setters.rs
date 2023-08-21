#![no_main]

wit_bindgen::generate!("import-setters");

export_import_setters!(Implementation);

use self::{
    exports::witty_macros::test_modules::entrypoint::Entrypoint,
    witty_macros::test_modules::setters::*,
};

struct Implementation;

impl Entrypoint for Implementation {
    fn entrypoint() {
        set_bool(false);
        set_s8(-100);
        set_u8(201);
        set_s16(-20_000);
        set_u16(50_000);
        set_s32(-2_000_000);
        set_u32(4_000_000);
        set_float32(10.4);
        set_float64(-0.000_08);
    }
}
