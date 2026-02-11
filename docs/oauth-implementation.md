# OAuth Implementation Guide

This document explains the Google OAuth implementation for the anime-calendar application.

## Overview

The application now supports OAuth login through Google, allowing users to authenticate using their existing social accounts instead of creating new credentials.

## Backend Implementation

### Services

1. **Google OAuth Service** (`server/src/services/google_oauth.rs`)
   - Handles Google ID token verification
   - Extracts user information from verified tokens
   - Provides mock implementation for demonstration

### Controllers

1. **OAuth Controller** (`server/src/controllers/oauth.rs`)
   - `/auth/google` - Google OAuth endpoint
   - The endpoint verifies tokens and creates/logs in users

## Frontend Implementation

### Components

1. **GoogleLoginButton.vue**
   - Shows Google OAuth button with proper styling
   - Demonstrates the OAuth flow (redirect → callback → token exchange)

3. **LoginPage.vue**
   - Integrated OAuth buttons alongside regular login
   - Maintains existing login functionality

## OAuth Flow

### Google OAuth Flow

1. User clicks "Continue with Google"
2. Frontend redirects to Google OAuth consent screen
3. User authenticates and grants permissions
4. Google redirects back to callback URL with authorization code
5. Backend exchanges code for ID token
6. Backend verifies token and creates/updates user
7. Backend returns JWT token for session

## Security Considerations

1. **Token Verification**: All OAuth tokens must be properly verified using official APIs
2. **HTTPS**: All OAuth flows must use HTTPS in production
3. **Token Storage**: Access tokens should be stored securely
4. **Rate Limiting**: Implement rate limiting for OAuth endpoints
5. **Error Handling**: Proper error handling for invalid tokens

## Configuration Requirements

### Server Configuration

Add to `config.toml`:
```toml
[oauth.google]
client_id = "your-google-client-id"
client_secret = "your-google-client-secret"
```

### Environment Variables

```bash
GOOGLE_CLIENT_ID=your-google-client-id
GOOGLE_CLIENT_SECRET=your-google-client-secret
```

## Implementation Notes

1. **Mock Implementation**: The current implementation uses mock functions for demonstration
2. **Real Implementation**: Production code would use proper OAuth libraries:
   - Google: `googleapis` or similar
3. **Database Integration**: OAuth users are stored in the same database as regular users
4. **Session Management**: JWT tokens are used for session management

## Testing

1. Run unit tests for OAuth services
2. Test OAuth endpoint integration
3. Verify user creation/update logic
4. Test error scenarios (invalid tokens, etc.)

## Future Improvements

1. Add proper error handling for network failures
2. Implement refresh token handling
3. Add OAuth provider selection
4. Implement better logging and monitoring
5. Add support for more OAuth providers