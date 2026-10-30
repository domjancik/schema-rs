# Access PINs

A native LAN host accepts an explicitly configured four-ASCII-digit `SCHEMA_TOKEN`, or an existing token of at least16 characters. A LAN bind without either is rejected. Credentials are sent using the existing `x-schema-token` header, not in URLs. Long-token behavior remains compatible.

Four-digit guesses are limited to five failures per actual TCP client IP per minute and32 failures across clients. Forwarded-address headers do not change peer identity. Missing or stale long tokens cannot guess a four-digit key and do not consume its allowance. Successful authentication clears that peer’s failure window; expired attempts are pruned without permanent lockout. Requests during lockout return429 with Retry-After60.

The optional login portal accepts configured four- or six-digit PINs and renders the matching form length. Existing six-digit PINs remain supported; new generated PINs have four digits. Its CSRF challenge, IP-bound sessions and login limits remain enabled. Actual credentials belong in private operator storage and host configuration, never source or pairing QR codes.

Wi-Fi passphrases are separate from application PINs. WPA2 requires at least eight characters; provisioning accepts that minimum without making the network open. Native builds predating optional PIN support require a long token again if an operator chooses to downgrade.
