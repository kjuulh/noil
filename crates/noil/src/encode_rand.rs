pub(crate) const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";

pub(crate) const ALPHABET_LEN: u32 = ALPHABET.len() as u32;

pub(crate) fn encode_256bit_base36(input: &[u8; 32]) -> String {
    let mut num = *input;
    let mut output = Vec::with_capacity(52); // log_36(2^256) ≈ 50.7

    while num.iter().any(|&b| b != 0) {
        let mut rem: u32 = 0;
        for byte in num.iter_mut() {
            let acc = ((rem as u16) << 8) | *byte as u16;
            *byte = (acc / ALPHABET_LEN as u16) as u8;
            rem = (acc % ALPHABET_LEN as u16) as u32;
        }
        output.push(ALPHABET[rem as usize]);
    }

    if output.is_empty() {
        output.push(ALPHABET[0]);
    }

    output.reverse();
    String::from_utf8(output).unwrap()
}
