#![no_main]

use benchproof::{decode_run, explain_run};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let input = String::from_utf8_lossy(data);
    if let Ok(run) = decode_run(&input) {
        let _ = explain_run(&run);
    }
});
