export interface RunOutputAction {
  name: string;
  payload: object;
}

export interface RunOutput {
  id: number;
  context: object;
  actions: RunOutputAction[];
}

interface FrameMessage {
  id?: number;
  name: string;
  data: any;
}

interface Pending {
  reject: (e: Error) => void;
  resolve: (data: any) => void;
}

let msgId = 1;

export function sandbox(element: HTMLIFrameElement, handlers: Record<string, (data: any) => void>) {
  const pending = new Map<number, Pending>();

  function handleFrameMessage(evt: MessageEvent<FrameMessage>) {
    if (evt.source !== element.contentWindow) {
      return;
    }

    const msg = evt.data;

    if (msg.id) {
      let handler = pending.get(msg.id);
      if (!handler) {
        console.error('Received message for unknown id ' + msg.id);
        return;
      }

      pending.delete(msg.id);

      if (msg.name === 'respond_resolve') {
        let { stack, message } = msg.data;

        // Reconstruct the error from the other side.
        let e = new Error(message);
        e.stack = stack;

        handler.reject(e);
      } else if (msg.name === 'respond_reject') {
        pending.delete(msg.id);
        handler.resolve(msg.data);
      }
    } else {
      handlers[msg.name]?.(msg.data);
    }
  }

  window.addEventListener('message', handleFrameMessage);

  return {
    sendMessage(message: string, data: any) {
      let id = msgId++;

      return new Promise((resolve, reject) => {
        pending.set(id, { resolve, reject });
        element.contentWindow?.postMessage({ name: message, data, id }, '*');
      });
    },
    destroy() {
      window.removeEventListener('message', handleFrameMessage);
    },
  };
}
