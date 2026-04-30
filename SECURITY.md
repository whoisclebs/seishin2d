# Security Policy

`seishin2d` is an early engine prototype. Security-sensitive areas are still small but important:

- C ABI / FFI pointer and ownership boundaries;
- asset path validation and root containment;
- unsafe renderer surface creation from platform window handles;
- panic containment across FFI.

## Supported Versions

No stable release line exists yet. Please report issues against `main`.

## Reporting a Vulnerability

Please open a private security advisory if the repository host supports it. Otherwise, open an issue with minimal public detail and indicate that it is security-sensitive.

Do not include exploit payloads or sensitive machine-specific data in public issues.

## Expectations

Security fixes should include tests where practical, especially for:

- null/invalid FFI arguments;
- asset path traversal;
- panic-to-status conversion;
- safe handling of unavailable backends.
