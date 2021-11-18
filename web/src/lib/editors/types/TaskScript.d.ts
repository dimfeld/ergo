declare namespace Ergo {
  function getPayload<PAYLOAD>(): PAYLOAD;

  function runAction<ACTIONPAYLOAD>(name: string, payload: ACTIONPAYLOAD): void;

  function getContext<CONTEXT>(): CONTEXT | undefined;
  function setContext<CONTEXT>(context: CONTEXT): void;
}
