fn main() {
    let encrypted = "enc:PTYjLCUpMi0tNw==";
    let data_str = &encrypted[4..];
    
    // AlouetteSecretKey2026
    let encryption_key = b"AlouetteSecretKey2026";
    let decoded = base64::decode(data_str).unwrap();
    let mut plain = Vec::with_capacity(decoded.len());
    for (i, byte) in decoded.iter().enumerate() {
        plain.push(byte ^ encryption_key[i % encryption_key.len()]);
    }
    println!("Old plain: {}", String::from_utf8(plain).unwrap());
}
