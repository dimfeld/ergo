import { Handle } from '@sveltejs/kit';

const apiServer = new URL(
  `http://${process.env.API_HOST || 'localhost'}:${process.env.API_PORT || 6543}`
);
const apiKey = process.env.API_KEY;

export const handle: Handle = async ({ event, resolve }) => {
  if (event.url.pathname.startsWith('/api')) {
    event.url.host = apiServer.host;
    event.url.protocol = apiServer.protocol;
    if (apiKey) {
      event.request.headers.set('Authorization', 'Bearer ' + apiKey);
    }

    let req = new Request(event.url.toString(), event.request);
    return fetch(req);
  }

  // TODO Reenable SSR in everything except the editor, which will show a placeholder
  return resolve(event, { ssr: false });
};
