const crypto = require('crypto');

let x =  "{\"content\":\"\",\"created_at\":1678375933,\"id\":\"156799111eb6b541a0a657ba2ca3de80bd50e4a71381a256d8f68d1c4a38e901\",\"kind\":9735,\"pubkey\":\"ac77f432ab2d50c14fa5657783e090efe985adb26f6515060fbe95f3ba9f9e52\",\"sig\":\"b75dbaab3c26a6a5733f603ed02ab27e6b013854d9efb80ba4621fc51a5e70e7bb5dd733ff6866f56b7d027424cd2cfc8322c52d5ee6d43c9a63d874e34c8ebc\",\"tags\":[[\"p\",\"339e87a4ad5b32a7fa88c6a56a65a53bcfbea8595a86b5d87201a39b0408c29c\"],[\"e\",\"a6c376f4959a1fdffee15abb7b31845a52fb05470e72e0bb9f760bb79e31a4cc\"],[\"bolt11\",\"lnbcrt10u1pjqn7lnpp5qhkrz6f6srlqdjuemjrcvdhm6qdjpa5qqxlzfj3mrewqlzpe3ddqhp5v89fkfna8nrxm4cf774m47yc684vqupal93g587qmj42ufxm64escqzpgxqzfvsp5qkqygwepgxr07wexw99z5aqtn69fe8a3fvdnd0jpqlfgdpayrtxq9qyyssqp6ljqhdeh4a9rhfmxw0t70hmze5ypzs7f5nfkfpe9kwq8plncevrhzv4grv7kv6nyqjgcgzwm0m9j66j9c4ecu9x7e7dtqm90gwa9sgqryw80q\"],[\"preimage\",\"26f3d6333a68e52094b6c16398da9003c57610de1372e26ae119dac0a11dddba\"],[\"description\",\"{\\\"content\\\":\\\"\\\",\\\"created_at\\\":1677859275,\\\"id\\\":\\\"156799111eb6b541a0a657ba2ca3de80bd50e4a71381a256d8f68d1c4a38e901\\\",\\\"kind\\\":9734,\\\"pubkey\\\":\\\"339e87a4ad5b32a7fa88c6a56a65a53bcfbea8595a86b5d87201a39b0408c29c\\\",\\\"sig\\\":\\\"c3c1d14e77f7bee2166a46fbefdbaeead25ec57668082f395bf4ef049c80d61b80b6e45de9606c62838e61c2ab749e0f3e5b97cdea13ff38a15bda4230421a24\\\",\\\"tags\\\":[[\\\"e\\\",\\\"a6c376f4959a1fdffee15abb7b31845a52fb05470e72e0bb9f760bb79e31a4cc\\\"],[\\\"p\\\",\\\"339e87a4ad5b32a7fa88c6a56a65a53bcfbea8595a86b5d87201a39b0408c29c\\\"],[\\\"relays\\\",\\\"wss://relay.damus.io\\\",\\\"wss://nostr.foundrydigital.com\\\",\\\"wss://lnbits.eldamar.icu/nostrrelay/relay\\\",\\\"wss://eden.nostr.land\\\"]]}\"]]}";

let y = JSON.parse(x);

console.log(y);
// Generate a new key pair
const { privateKey, publicKey } = crypto.generateKeyPairSync('ec', {
  namedCurve: 'secp256k1',
});

// Convert the private key to hex format
const privateKeyHex = privateKey.export({ format: 'der', type: 'pkcs8' })
  .toString('hex')
  .slice(38, 70);

// Convert the public key to hex format
const publicKeyHex = publicKey.export({ format: 'der', type: 'spki' })
  .toString('hex')
  .slice(46);

console.log('Private key:', privateKeyHex);
console.log('Public key:', publicKeyHex);
