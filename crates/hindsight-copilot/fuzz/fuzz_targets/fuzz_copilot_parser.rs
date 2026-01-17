#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // TODO: Fuzz Copilot log parsing
    let _ = std::hint::black_box(data);
});
