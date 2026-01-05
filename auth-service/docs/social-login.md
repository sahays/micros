# Social Login Integration Guide

This guide explains how to integrate Google OAuth 2.0 with the Auth Service. The implementation uses the Authorization Code Flow with PKCE (Proof Key for Code Exchange) for enhanced security.

## 1. Configuration

Ensure the following environment variables are set in `.env`:

```bash
GOOGLE_CLIENT_ID=your-google-client-id
GOOGLE_CLIENT_SECRET=your-google-client-secret
GOOGLE_REDIRECT_URI=http://localhost:3000/auth/google/callback
```

### Setting up Google Cloud Console

1.  Go to the [Google Cloud Console](https://console.cloud.google.com/).
2.  Create a new project or select an existing one.
3.  Navigate to **APIs & Services > Credentials**.
4.  Create **OAuth 2.0 Client IDs**.
5.  Set Application Type to **Web application**.
6.  Add Authorized Redirect URIs: matches `GOOGLE_REDIRECT_URI` (e.g., `http://localhost:3000/auth/google/callback`).

## 2. Authentication Flow

### Step 1: Initiate Login

Redirect the user's browser to the initiation endpoint.

`GET /auth/google`

**What happens server-side:**
1.  Generates a random `state` and PKCE `code_verifier`.
2.  Stores these in secure, HTTP-only cookies (`oauth_state`, `code_verifier`).
3.  Redirects the user to Google's consent screen.

### Step 2: User Consent

The user logs in with their Google account and grants permission to access their profile and email (`openid email profile`).

### Step 3: Callback Processing

Google redirects the user back to your application:

`GET /auth/google/callback?code=...&state=...`

**What happens server-side:**
1.  Validates the `state` against the `oauth_state` cookie.
2.  Exchanges the `code` and `code_verifier` cookie for a Google Access Token.
3.  Fetches user profile (email, name) from Google.
4.  **Auto-Registration:** If the email doesn't exist, a new user is created with a verified status.
5.  **Session Creation:** Issues a standard JWT Access/Refresh Token pair.
6.  **Cleanup:** Removes temporary OAuth cookies.

**Response:**
Returns the standard Token Response (same as password login):
```json
{
  "access_token": "eyJ...",
  "refresh_token": "eyJ...",
  "token_type": "Bearer",
  "expires_in": 900
}
```

## 3. Frontend Integration Scenarios

### Scenario A: Full Page Redirect (Simplest)

1.  Add a "Login with Google" link pointing directly to `https://auth-service.com/auth/google`.
2.  The browser follows the redirects.
3.  Upon success, the JSON response containing the tokens will be displayed in the browser.
    *   *Note:* In a real app, you likely want the callback to redirect *again* to your frontend app with the tokens, or use a popup.

### Scenario B: Popup / Window (Recommended)

1.  Open `https://auth-service.com/auth/google` in a new popup window.
2.  The user completes the flow in the popup.
3.  The final JSON response is rendered in the popup.
4.  **Enhancement needed:** Currently, the backend returns JSON. To support this flow better, the backend endpoint `google_callback` should ideally be modified to *redirect* to a frontend URL with tokens as query params, or render a small HTML script that uses `window.opener.postMessage` to send tokens back to the main window.

    *Current implementation returns JSON, so the frontend would need to manually parse the response if fetching via AJAX, but CORS and Redirects make AJAX complicated for OAuth. Standard practice is a full redirect.*

## 4. Security Notes

-   **PKCE:** We use PKCE to prevent authorization code injection attacks.
-   **State:** The `state` parameter prevents CSRF attacks during the OAuth flow.
-   **Cookies:** Temporary secrets are stored in `HttpOnly; Secure; SameSite=Lax` cookies.
