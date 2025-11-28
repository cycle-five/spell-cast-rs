import { DiscordSDK } from '@discord/embedded-app-sdk';

let discordSdk = null;
let isDiscordActivity = false;

// Helper to get the correct API base URL
export function getApiUrl(path) {
  // When running inside Discord Activity, use the proxy path directly
  if (isDiscordActivity) {
    return `/.proxy${path}`;
  }
  return path;
}

export async function initDiscord() {
  // Get client ID from environment or config
  const clientId = import.meta.env.VITE_DISCORD_CLIENT_ID;
  if (!clientId) {
    if (import.meta.env.DEV) {
      console.warn('VITE_DISCORD_CLIENT_ID is not set. Using mock client ID in development mode.');
    } else {
      throw new Error('Discord client ID is missing. Please set VITE_DISCORD_CLIENT_ID in your environment.');
    }
  }

  discordSdk = new DiscordSDK(clientId || 'YOUR_CLIENT_ID');

  try {
    // Wait for Discord to be ready
    await discordSdk.ready();
    console.log('Discord client is ready');

    // Detect if we're running inside Discord Activity
    isDiscordActivity = window.location.host.includes('discordsays.com');
    console.log('Running in Discord Activity:', isDiscordActivity);

    // Authorize the app
    const { code } = await discordSdk.commands.authorize({
      client_id: clientId,
      response_type: 'code',
      state: '',
      prompt: 'none',
      scope: [
        'identify',
        'guilds',
      ],
    });

    console.log('Authorization code received');

    // Exchange code for access token via backend
    // The backend exchanges with Discord and returns both the Discord access token
    // (for SDK authentication) and our JWT (for backend API calls)
    const response = await fetch(getApiUrl('/api/auth/exchange'), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ code }),
    });

    if (!response.ok) {
      const errorText = await response.text();
      console.error('Exchange failed:', response.status, errorText);
      throw new Error(`Failed to exchange authorization code: ${response.status}`);
    }

    const { access_token, discord_access_token } = await response.json();
    console.log('Tokens received from backend');

    // Authenticate with Discord using the DISCORD access token (not our JWT)
    // This completes the OAuth flow with Discord's SDK
    const auth = await discordSdk.commands.authenticate({
      access_token: discord_access_token,
    });

    console.log('Authenticated with Discord:', auth.user);

    // Get channel and guild context from the SDK
    // These are available after the SDK is ready
    const channelId = discordSdk.channelId;
    const guildId = discordSdk.guildId;

    console.log('Discord context - Channel:', channelId, 'Guild:', guildId);

    return {
      sdk: discordSdk,
      user: auth.user,
      // Return our JWT for backend API authentication
      access_token,
      discord_access_token,
      // Return channel/guild context for lobby scoping
      channelId,
      guildId,
    };
  } catch (error) {
    console.error('Discord initialization error:', error);

    // In development, we can continue without Discord
    if (import.meta.env.DEV) {
      console.warn('Running in development mode without Discord SDK');
      return {
        sdk: null,
        user: {
          id: 'dev_user',
          username: 'Developer',
          discriminator: '0000',
        },
        access_token: 'dev_token',
        // Use a dev channel ID for local testing
        channelId: 'dev_channel_123',
        guildId: 'dev_guild_456',
      };
    }

    throw error;
  }
}

export function getDiscordSdk() {
  return discordSdk;
}
