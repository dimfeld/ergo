import { Handle } from '@sveltejs/kit';

const apiServer = `http://localhost:${process.env.API_PORT || 6543}`;
const apiKey = process.env.API_KEY;

export const handle: Handle = async ({ request, resolve }) => {
  if (!request.host && request.path.startsWith('/api')) {
    let url = apiServer + request.path;
    if (request.query) {
      url += '?' + request.query.toString();
    }

    let result = await fetch(url, {
      method: request.method,
      body: request.rawBody,
      headers: {
        ...request.headers,
        Authorization: 'Bearer ' + apiKey,
      },
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
