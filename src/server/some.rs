use dotenv::dotenv;
use secp256k1::{schnorr, KeyPair, Message, PublicKey, Secp256k1, SecretKey};

pub fn some(msg: &str) {
    dotenv::dotenv().ok();

    let privkey = std::env::var("NOSTR_PRIVATE_KEY").unwrap();

    let secp = Secp256k1::new();
    let secret_key =
        SecretKey::from_slice(&hex::decode(privkey).expect("FailedToDecodeHexPrivateKey"))
            .expect("32 bytes, within curve order");
    let (xpub, _) = PublicKey::from_secret_key(&secp, &secret_key).x_only_public_key();
    let pair = KeyPair::from_seckey_slice(&secp, &secret_key.secret_bytes())
        .expect("Failed to generate keypair from secret key");
    // This is unsafe unless the supplied byte slice is the output of a cryptographic hash function.
    // See the above example for how to use this library together with `bitcoin-hashes-std`.
    let message =
        Message::from_slice(&hex::decode(msg).expect("UnableToDecodeHexMessageForSigning"))
            .expect("FailedToConvertHexMessageToBytes");

    // let sig = secp.sign_ecdsa(&message, &secret_key);
    let sig = secp.sign_schnorr_no_aux_rand(&message, &pair);

    assert!(secp.verify_schnorr(&sig, &message, &xpub).is_ok());
}
