import init from 'ergo-wasm';
import wasmUrl from 'ergo-wasm/ergo_wasm_bg.wasm?url';
import once from 'just-once';

export default once(() => init(wasmUrl));
