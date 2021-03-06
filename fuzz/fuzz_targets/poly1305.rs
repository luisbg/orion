#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate orion;
pub mod util;

use self::util::*;
use orion::hazardous::poly1305::*;

fuzz_target!(|data: &[u8]| {
    let mut key = [0u8; 32];
    apply_from_input_fixed(&mut key, &data, 0);

    let mut message = Vec::new();
    apply_from_input_heap(&mut message, data, key.len());

    // Test both stream and one-shot
    let mut poly1305_state = init(&key).unwrap();
    poly1305_state.update(&message).unwrap();
    let orion_stream_tag = poly1305_state.finalize().unwrap();
    let orion_oneshot_tag = poly1305(&key, &message).unwrap();

    assert_eq!(orion_stream_tag, orion_oneshot_tag);
    assert!(verify(&orion_stream_tag, &key, &message).unwrap());
    assert!(verify(&orion_oneshot_tag, &key, &message).unwrap());
});
