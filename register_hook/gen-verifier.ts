function generateRandomString(length: number): string {
    let randomString = '';
    const characters = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    const charactersLength = characters.length;
    for (let i = 0; i < length; i++) {
        randomString += characters.charAt(Math.floor(Math.random() * charactersLength));
    }
    return randomString;
}

// Generate two random strings
const a = generateRandomString(16); // You can adjust the length as needed
const b = generateRandomString(16); // You can adjust the length as needed

// Concatenate the random strings to get a longer string
const randStr = a.concat(b);

// Encode the concatenated string as a Uint8Array
const textEncoder = new TextEncoder();
const encodedData = textEncoder.encode(randStr);

// Hash the concatenated string using SHA256
const hashBuffer = await crypto.subtle.digest('SHA-256', encodedData);
const hashArray = Array.from(new Uint8Array(hashBuffer)); // Convert buffer to byte array
const hashHex = hashArray.map(byte => byte.toString(16).padStart(2, '0')).join(''); // Convert bytes to hex string

// Log the random string and the verifier string to the console
console.log(`Random String: ${randStr}`);
console.log(`Code Verifier: ${hashHex}`);