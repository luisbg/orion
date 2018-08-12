// MIT License

// Copyright (c) 2018 brycx

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use byte_tools::write_u32_be;
use hazardous::constants::{HLenArray, HLEN};
use hazardous::hmac::*;
use utilities::{errors::*, util};

/// PBKDF2 (Password-Based Key Derivation Function 2) as specified in the
/// [RFC 8018](https://tools.ietf.org/html/rfc8018).
///
/// Fields `password` and `salt` are zeroed out on drop.

/// PBKDF2 (Password-Based Key Derivation Function 2) as specified in the
/// [RFC 8018](https://tools.ietf.org/html/rfc8018).
///
/// # Parameters:
/// - `password`: Password
/// - `salt`: Salt value
/// - `iterations`: Iteration count
/// - `okm_out`: Destination buffer for the derivec key. The length of the derived key is implied by the length of `okm_out`
///
/// See [RFC](https://tools.ietf.org/html/rfc8018#section-5.2) for more information.
///
/// # Exceptions:
/// An exception will be thrown if:
/// - The specified dklen is less than 1
/// - The specified dklen is greater than (2^32 - 1) * hLen
/// - The specified iteration count is less than 1
///
/// # Security:
/// Salts should always be generated using a CSPRNG. The `gen_rand_key` function
/// in `util` can be used for this. The recommended length for a salt is 16 bytes as a minimum.
/// The iteration count should be set as high as feasible.
/// # Example:
/// ### Generating derived key:
/// ```
/// use orion::hazardous::pbkdf2;
/// ```
/// ### Verifying derived key:
/// ```
/// use orion::hazardous::pbkdf2;
/// ```

#[inline(always)]
fn function_f(salt: &[u8], iterations: usize, index: usize, dk_block: &mut [u8], hmac: &mut Hmac) {
    let block_len = dk_block.len();

    let mut u_step: HLenArray = [0u8; 64];
    // First 4 bytes used for index BE conversion
    write_u32_be(&mut u_step[..4], index as u32);
    hmac.update(salt);
    hmac.update(&u_step[..4]);

    hmac.finalize_with_dst(&mut u_step);
    dk_block.copy_from_slice(&u_step[..block_len]);

    if iterations > 1 {
        for _ in 1..iterations {
            hmac.reset();
            hmac.update(&u_step);
            hmac.finalize_with_dst(&mut u_step);

            for (idx, val) in u_step[..block_len].iter().enumerate() {
                dk_block[idx] ^= val;
            }
        }
    }
}

#[inline(always)]
pub fn derive_key(
    password: &[u8],
    salt: &[u8],
    iterations: usize,
    dk_out: &mut [u8],
) -> Result<(), UnknownCryptoError> {
    if iterations < 1 {
        return Err(UnknownCryptoError);
    }
    if dk_out.len() > 274_877_906_880 {
        return Err(UnknownCryptoError);
    }
    if dk_out.len() < 1 {
        return Err(UnknownCryptoError);
    }

    let mut hmac = init(password);

    for (idx, dk_block) in dk_out.chunks_mut(HLEN).enumerate() {
        function_f(salt, iterations, idx + 1, dk_block, &mut hmac);
    }

    Ok(())
}

/// Verify a derived key by comparing one from the current struct fields with the derived key
/// passed to the function. Comparison is done in constant time. Both derived keys must be
/// of equal length.
pub fn verify(
    expected_dk: &[u8],
    password: &[u8],
    salt: &[u8],
    iterations: usize,
    dk_out: &mut [u8],
) -> Result<bool, ValidationCryptoError> {
    derive_key(password, salt, iterations, dk_out).unwrap();

    if util::compare_ct(&dk_out, expected_dk).is_err() {
        Err(ValidationCryptoError)
    } else {
        Ok(true)
    }
}

#[cfg(test)]
mod test {

    extern crate hex;
    use self::hex::decode;
    use hazardous::pbkdf2::*;

    #[test]
    fn zero_iterations_err() {
        let password = "password".as_bytes();
        let salt = "salt".as_bytes();
        let iterations: usize = 0;
        let mut okm_out = [0u8; 15];

        assert!(derive_key(password, salt, iterations, &mut okm_out).is_err());
    }

    #[test]
    fn zero_dklen_err() {
        let password = "password".as_bytes();
        let salt = "salt".as_bytes();
        let iterations: usize = 1;
        let mut okm_out = [0u8; 0];

        assert!(derive_key(password, salt, iterations, &mut okm_out).is_err());
    }

    #[test]
    fn verify_true() {
        let password = "pass\0word".as_bytes();
        let salt = "sa\0lt".as_bytes();
        let iterations: usize = 4096;
        let mut okm_out = [0u8; 16];

        let expected_dk = decode("9d9e9c4cd21fe4be24d5b8244c759665").unwrap();

        assert_eq!(
            verify(&expected_dk, password, salt, iterations, &mut okm_out).unwrap(),
            true
        );
    }

    #[test]
    fn verify_false_wrong_salt() {
        let password = "pass\0word".as_bytes();
        let salt = "".as_bytes();
        let iterations: usize = 4096;
        let mut okm_out = [0u8; 16];

        let expected_dk = decode("9d9e9c4cd21fe4be24d5b8244c759665").unwrap();

        assert!(verify(&expected_dk, password, salt, iterations, &mut okm_out).is_err());
    }
    #[test]
    fn verify_false_wrong_password() {
        let password = "".as_bytes();
        let salt = "sa\0lt".as_bytes();
        let iterations: usize = 4096;
        let mut okm_out = [0u8; 16];

        let expected_dk = decode("9d9e9c4cd21fe4be24d5b8244c759665").unwrap();

        assert!(verify(&expected_dk, password, salt, iterations, &mut okm_out).is_err());
    }

    #[test]
    fn verify_diff_dklen_error() {
        let password = "pass\0word".as_bytes();
        let salt = "sa\0lt".as_bytes();
        let iterations: usize = 4096;
        let mut okm_out = [0u8; 32];

        let expected_dk = decode("9d9e9c4cd21fe4be24d5b8244c759665").unwrap();

        assert!(verify(&expected_dk, password, salt, iterations, &mut okm_out).is_err());
    }

    #[test]
    fn verify_diff_iter_error() {
        let password = "pass\0word".as_bytes();
        let salt = "sa\0lt".as_bytes();
        let iterations: usize = 512;
        let mut okm_out = [0u8; 16];

        let expected_dk = decode("9d9e9c4cd21fe4be24d5b8244c759665").unwrap();

        assert!(verify(&expected_dk, password, salt, iterations, &mut okm_out).is_err());
    }
}
