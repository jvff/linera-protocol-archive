static mut COUNTER: u32 = 0;

#[no_mangle]
pub fn example(input: u32) -> u32 {
    unsafe {
        COUNTER += 1;
        input + COUNTER
    }
}
