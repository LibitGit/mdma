//TODO: Implement actual caching behavior.
const cacheToken = Date.now();
const src = decodeURIComponent(window.location.hash.slice(1));

const initJS = new Promise((res, rej) => import(src + `/popup.js?c=${cacheToken}`).then(res).catch(rej));

//TODO: Use encoded version in release.
const module_or_path = { module_or_path: src + `/popup_bg.wasm?c=${cacheToken}` };
const init = new Promise((res, rej) => initJS.then(module => module.default(module_or_path).then(() => res(module))).catch(rej));
const jsHandleMessage = (a, b, c) => init.then(({handleMessage}) => handleMessage(a, b, c).catch(console.error)).catch(console.error);

chrome.runtime.onMessage.addListener(jsHandleMessage);
