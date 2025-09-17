# Chrome Extension ID Calculation

This document explains how Chrome calculates extension IDs from public keys and provides a script to calculate the ID for any given public key.

## How Chrome Calculates Extension IDs

Chrome extension IDs are deterministically generated from the extension's public key using the following process:

1. **Take the SHA-256 hash** of the public key (in DER format, base64-encoded in the manifest)
2. **Take the first 32 characters** of the hexadecimal representation of that hash
3. **Translate characters** using this mapping:
   - `0` → `a`
   - `1` → `b`
   - `2` → `c`
   - `3` → `d`
   - `4` → `e`
   - `5` → `f`
   - `6` → `g`
   - `7` → `h`
   - `8` → `i`
   - `9` → `j`
   - `a` → `k`
   - `b` → `l`
   - `c` → `m`
   - `d` → `n`
   - `e` → `o`
   - `f` → `p`

## Prerequisites

To run the calculation script, you'll need Python 3 installed on your system.

## Calculate Extension ID

Create a Python script named `calculate_extension_id.py`:

```python
import hashlib
import base64

def calculate_extension_id(public_key_base64):
    """
    Calculate the Chrome extension ID from a base64-encoded public key.
    
    Args:
        public_key_base64 (str): The base64-encoded public key from manifest.json
        
    Returns:
        str: The 32-character extension ID
    """
    # Decode the Base64 public key
    decoded_key = base64.b64decode(public_key_base64)
    
    # Calculate SHA256 hash
    sha256_hash = hashlib.sha256(decoded_key).hexdigest()
    
    # Take first 32 characters
    first_32 = sha256_hash[:32]
    
    # Translate 0-9,a-f to a-p
    def translate_char(c):
        if '0' <= c <= '9':
            return chr(ord(c) + 49)  # 0->a, 1->b, etc.
        elif 'a' <= c <= 'f':
            return chr(ord(c) + 10)  # a->k, b->l, etc.
        return c

    translated = ''.join(translate_char(c) for c in first_32)
    return translated

# Example usage
if __name__ == "__main__":
    # Replace this with your public key from manifest.json
    public_key = "MEkwDQYJKoZIhvcNAQEBBQADOAAwNQJAMbU/inVAYtwmIZesaaHTAruXWsJDNL+Vcagg4eaUD/XesUvWd5Xjdv4Vj2NnC9u2vlcXEvOiYs2DZG+80CipxQIDAQAB"
    
    extension_id = calculate_extension_id(public_key)
    print(f"Public Key: {public_key}")
    print(f"Extension ID: {extension_id}")
```

## Usage

1. Save the script above to a file named `calculate_extension_id.py`
2. Replace the `public_key` variable with your actual public key from `manifest.json`
3. Run the script:
   ```bash
   python3 calculate_extension_id.py
   ```

## Using the Vanity ID Generator

When you run the vanity ID generator tool, it will produce several output files:

1. `public_key.der` - The public key in DER (binary) format
2. `public_key.pem` - The public key in PEM format (base64-encoded DER with headers)

Additionally, the tool will print the base64-encoded public key directly to the console, which you can easily copy and paste into your Chrome extension's `manifest.json` file:

```json
{
  "key": "BASE64_ENCODED_PUBLIC_KEY_HERE",
  "name": "Your Extension",
  "version": "1.0",
  "manifest_version": 3
}
```

The base64-encoded public key is the value you need for the "key" field in your manifest.json file.

## Example

For the public key:
```
MEkwDQYJKoZIhvcNAQEBBQADOAAwNQJAMbU/inVAYtwmIZesaaHTAruXWsJDNL+Vcagg4eaUD/XesUvWd5Xjdv4Vj2NnC9u2vlcXEvOiYs2DZG+80CipxQIDAQAB
```

The calculated extension ID would be:
```
eppeofbfhpjhlhgebcmpfcdcpaepoink
```

Note that this matches exactly what Chrome generates for this public key.

## Generating Vanity Extension IDs

If you want to generate an extension with a specific ID (vanity ID), you would need to:

1. Generate many key pairs
2. Calculate the extension ID for each public key
3. Check if it matches your desired pattern
4. Repeat until you find a match

This process can be automated but may require generating thousands or millions of keys depending on how specific your desired pattern is.