#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // TODO: Fuzz test result parsing
    let _ = std::hint::black_box(data);
});
