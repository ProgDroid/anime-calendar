# OAuth Implementation

Google OAuth is implemented using the `google-oauth` crate for server-side ID token verification.

## Flow

1. User clicks "Continue with Google" in `GoogleLoginButton.vue`
2. Frontend initiates the Google OAuth flow
3. Google redirects back with an authorization code / ID token
4. Frontend sends the token to `POST /auth/google`
5. Backend verifies the ID token via `google-oauth` crate using the configured `google_client_id`
6. Backend creates or updates the user record, then issues a JWT session token
7. Frontend stores the JWT and proceeds as with a regular login

## Backend

- **Controller**: `server/src/controllers/oauth.rs` — handles `POST /auth/google`
- **Service**: `server/src/services/google_oauth.rs` — token verification logic using `google-oauth` crate
- **Mapper**: `server/src/mappers/google_oauth.rs` — user creation/lookup from OAuth claims

OAuth users are stored in the same `users` table as regular users. JWT tokens are used for all subsequent session management.

## Configuration

`config.toml`:
```toml
google_client_id = "your-google-client-id"
```

The client secret is not needed server-side — verification uses the public Google certs.

## Frontend

- `GoogleLoginButton.vue` — renders the OAuth button and handles the redirect/callback
- `LoginPage.vue` — integrates the Google button alongside regular login form

## Security Notes

- ID tokens are verified against Google's public certificates (not just decoded)
- All OAuth flows must use HTTPS in production
- JWT tokens are short-lived; no refresh token implementation yet
- Rate limiting for `/auth/google` is a known TODO
