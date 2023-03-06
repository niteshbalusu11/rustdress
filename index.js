const crypto = require('crypto');

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
