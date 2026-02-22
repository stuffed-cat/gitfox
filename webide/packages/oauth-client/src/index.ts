/**
 * GitFox OAuth2 Client for WebIDE
 * 
 * Handles OAuth2 PKCE flow for public clients (WebIDE).
 * In normal operation, the access token is passed from the parent window,
 * but this client can also be used for standalone authentication.
 */

export interface OAuthConfig {
  /** GitFox instance URL */
  gitfoxUrl: string;
  
  /** OAuth client ID (from admin-configured application) */
  clientId: string;
  
  /** Redirect URI for OAuth callback */
  redirectUri: string;
  
  /** Requested scopes */
  scopes: string[];
}

export interface TokenResponse {
  access_token: string;
  token_type: string;
  expires_in: number;
  refresh_token?: string;
  scope: string;
  created_at: number;
}

export interface OAuthState {
  codeVerifier: string;
  state: string;
  redirectUri: string;
}

const STORAGE_KEY = 'gitfox_oauth_state';

/**
 * Generate cryptographically random string
 */
function generateRandomString(length: number): string {
  const array = new Uint8Array(length);
  crypto.getRandomValues(array);
  return Array.from(array, byte => byte.toString(16).padStart(2, '0')).join('');
}

/**
 * Generate PKCE code verifier
 */
function generateCodeVerifier(): string {
  return generateRandomString(64);
}

/**
 * Generate PKCE code challenge from verifier
 */
async function generateCodeChallenge(verifier: string): Promise<string> {
  const encoder = new TextEncoder();
  const data = encoder.encode(verifier);
  const hash = await crypto.subtle.digest('SHA-256', data);
  
  // Base64url encode
  return btoa(String.fromCharCode(...new Uint8Array(hash)))
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '');
}

export class GitFoxOAuthClient {
  private config: OAuthConfig;

  constructor(config: OAuthConfig) {
    this.config = {
      ...config,
      gitfoxUrl: config.gitfoxUrl.replace(/\/$/, ''),
    };
  }

  /**
   * Start OAuth2 authorization flow
   * Redirects user to GitFox authorization page
   */
  async startAuthorization(): Promise<void> {
    const codeVerifier = generateCodeVerifier();
    const codeChallenge = await generateCodeChallenge(codeVerifier);
    const state = generateRandomString(32);

    // Save state for callback handling
    const oauthState: OAuthState = {
      codeVerifier,
      state,
      redirectUri: this.config.redirectUri,
    };
    sessionStorage.setItem(STORAGE_KEY, JSON.stringify(oauthState));

    // Build authorization URL
    const params = new URLSearchParams({
      client_id: this.config.clientId,
      redirect_uri: this.config.redirectUri,
      response_type: 'code',
      scope: this.config.scopes.join(' '),
      state,
      code_challenge: codeChallenge,
      code_challenge_method: 'S256',
    });

    const authUrl = `${this.config.gitfoxUrl}/oauth/authorize?${params}`;
    window.location.href = authUrl;
  }

  /**
   * Handle OAuth callback and exchange code for tokens
   */
  async handleCallback(callbackUrl: string): Promise<TokenResponse> {
    const url = new URL(callbackUrl);
    const code = url.searchParams.get('code');
    const state = url.searchParams.get('state');
    const error = url.searchParams.get('error');

    if (error) {
      const errorDescription = url.searchParams.get('error_description');
      throw new Error(`OAuth error: ${error} - ${errorDescription}`);
    }

    if (!code) {
      throw new Error('Missing authorization code');
    }

    // Retrieve saved state
    const savedStateJson = sessionStorage.getItem(STORAGE_KEY);
    if (!savedStateJson) {
      throw new Error('Missing OAuth state');
    }

    const savedState: OAuthState = JSON.parse(savedStateJson);

    if (state !== savedState.state) {
      throw new Error('Invalid OAuth state');
    }

    // Clear saved state
    sessionStorage.removeItem(STORAGE_KEY);

    // Exchange code for tokens
    return this.exchangeCode(code, savedState.codeVerifier, savedState.redirectUri);
  }

  /**
   * Exchange authorization code for tokens
   */
  private async exchangeCode(
    code: string,
    codeVerifier: string,
    redirectUri: string
  ): Promise<TokenResponse> {
    const response = await fetch(`${this.config.gitfoxUrl}/oauth/token`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
      },
      body: new URLSearchParams({
        grant_type: 'authorization_code',
        client_id: this.config.clientId,
        code,
        redirect_uri: redirectUri,
        code_verifier: codeVerifier,
      }),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Token exchange failed: ${error}`);
    }

    return response.json();
  }

  /**
   * Refresh access token
   */
  async refreshToken(refreshToken: string): Promise<TokenResponse> {
    const response = await fetch(`${this.config.gitfoxUrl}/oauth/token`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
      },
      body: new URLSearchParams({
        grant_type: 'refresh_token',
        client_id: this.config.clientId,
        refresh_token: refreshToken,
      }),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Token refresh failed: ${error}`);
    }

    return response.json();
  }

  /**
   * Revoke access or refresh token
   */
  async revokeToken(token: string, tokenType: 'access_token' | 'refresh_token' = 'access_token'): Promise<void> {
    const response = await fetch(`${this.config.gitfoxUrl}/oauth/revoke`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
      },
      body: new URLSearchParams({
        client_id: this.config.clientId,
        token,
        token_type_hint: tokenType,
      }),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Token revocation failed: ${error}`);
    }
  }

  /**
   * Check if current URL is an OAuth callback
   */
  isCallback(url: string = window.location.href): boolean {
    const urlObj = new URL(url);
    return urlObj.searchParams.has('code') || urlObj.searchParams.has('error');
  }
}

export function createOAuthClient(config: OAuthConfig): GitFoxOAuthClient {
  return new GitFoxOAuthClient(config);
}

export default GitFoxOAuthClient;
