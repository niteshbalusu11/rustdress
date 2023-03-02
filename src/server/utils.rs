use bech32::{encode, ToBase32, Variant};
use dotenv::dotenv;

pub fn get_identifiers() -> (String, String) {
    dotenv().ok();

    let domain = std::env::var("DOMAIN").unwrap();
    let username = std::env::var("USERNAME").unwrap();

    return (domain, username);
}

pub fn bech32_encode(prefix: String, data: String) -> Result<String, bech32::Error> {
    let base32_data = data.to_base32();

    let encoded = encode(&prefix, base32_data, Variant::Bech32);

    match encoded {
        Ok(_) => encoded,
        Err(_) => panic!("FailedToEncodeToBech32"),
    }
}

pub fn add_hop_hints() -> bool {
    let is_add_hints = std::env::var("INCLUDE_HOP_HINTS");

    match is_add_hints {
        Ok(add) => {
            if add == "true" {
                return true;
            }

            if add == "false" {
                return false;
            }

            false
        }

        Err(_) => false,
    }
}

pub fn buffer_as_hex(bytes: Vec<u8>) -> String {
    let hex_str = bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();

    return hex_str;
}
