import { Handle } from '@sveltejs/kit';

const apiServer = new URL(
  `http://${process.env.API_HOST || 'localhost'}:${process.env.API_PORT || 6543}`
);
const apiKey = process.env.API_KEY;

export const handle: Handle = async ({ request, resolve }) => {
  if (request.url.pathname.startsWith('/api')) {
    request.url.host = apiServer.host;
    request.url.protocol = apiServer.protocol;
    if (apiKey) {
      request.headers['Authorization'] = 'Bearer ' + apiKey;
    }

    let result = await fetch(request.url.toString(), {
      method: request.method,
      body: request.rawBody,
      headers: request.headers,
    });

    let body = await result.text();

    let response = {
      status: result.status,
      headers: Object.fromEntries(Array.from(result.headers.entries())),
      body,
    };

    return response;
  }

  return resolve(request);
};
