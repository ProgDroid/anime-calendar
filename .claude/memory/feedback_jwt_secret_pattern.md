---
name: JwtSecret expose_secret pattern
description: JwtSecret has a concrete expose_secret() method — importing secrecy::ExposeSecret trait is NOT needed and causes an unused-import warning
type: feedback
originSessionId: 9bdd3d50-f1c6-463c-9c96-c05ae6d37d9d
---
`JwtSecret` (defined in `server/src/config/server.rs`) is a newtype wrapping `SecretString`. It exposes a **concrete method** `expose_secret(&self) -> &str` that delegates to the inner `SecretString`:

```rust
pub struct JwtSecret(SecretString);

impl JwtSecret {
    pub fn expose_secret(&self) -> &str {
        self.0.expose_secret()
    }
}
```

**Why:** The method is directly on the struct, not via the `ExposeSecret` trait. Calling `jwt_secret.expose_secret()` does NOT require `use secrecy::ExposeSecret` in scope.

**How to apply:** In any controller or handler that calls `jwt_secret.expose_secret()`, do NOT add `use secrecy::ExposeSecret` — the compiler will warn "unused import". Only add `use secrecy::ExposeSecret` when calling `.expose_secret()` on a raw `SecretString` or `Secret<T>` directly.
