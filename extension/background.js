import __wbg_init, { handlePortConnect, handleActionClick, handleMessage } from "https://libit.ovh/mdma/background.js";

let module_or_path = {module_or_path: `https://libit.ovh/mdma/background_bg.wasm?c=${Date.now()}`};
let init = new Promise((res, rej) => __wbg_init(module_or_path).then(res).catch(rej));

init.then(() => console.log("bgInit")).catch(console.error)

let jsHandlePortConnect = port => init.then(() => handlePortConnect(port).catch(console.error)).catch(console.error);
let jsHandleMessage = (message, sender, sendResponse) => init.then(() => handleMessage(message, sender, sendResponse).catch(console.error)).catch(console.error);
let jsHandleActionClick = tab => init.then(() => handleActionClick(tab).catch(console.error)).catch(console.error);

chrome.runtime.onConnect.addListener(jsHandlePortConnect);
chrome.runtime.onConnectExternal.addListener(jsHandlePortConnect);
chrome.runtime.onMessage.addListener(jsHandleMessage);
chrome.runtime.onMessageExternal.addListener(jsHandleMessage);
chrome.action.onClicked.addListener(jsHandleActionClick)
