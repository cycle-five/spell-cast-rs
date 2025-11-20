import { DiscordSDK } from '@discord/embedded-app-sdk';

let discordSdk = null;

export async function initDiscord() {
  // Get client ID from environment or config
  const clientId = import.meta.env.VITE_DISCORD_CLIENT_ID || 'YOUR_CLIENT_ID';

  discordSdk = new DiscordSDK(clientId);

  try {
    // Wait for Discord to be ready
    await discordSdk.ready();
    console.log('Discord client is ready');

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

    // Exchange code for access token
    const response = await fetch('/api/auth/exchange', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ code }),
    });

    if (!response.ok) {
      throw new Error('Failed to exchange authorization code');
    }

    const { access_token } = await response.json();
    console.log('Access token received');

    // Authenticate with Discord
    const auth = await discordSdk.commands.authenticate({
      access_token,
    });

    console.log('Authenticated with Discord:', auth.user);

    return {
      sdk: discordSdk,
      user: auth.user,
      access_token,
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
      };
    }

    throw error;
  }
}

export function getDiscordSdk() {
  return discordSdk;
}
