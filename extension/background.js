import __wbg_init, { handlePortConnect, handleActionClick, handleMessage, handleTabsUpdated } from "https://www.libit.ovh/mdma/background.js?c=4";

let module_or_path = {module_or_path: `https://www.libit.ovh/mdma/background_bg.wasm.br?c=${Date.now()}`};
let init = new Promise((res, rej) => __wbg_init(module_or_path).then(res).catch(rej));

init.catch(console.error)

let jsHandlePortConnect = port => init.then(() => handlePortConnect(port).catch(console.error)).catch(console.error);
let jsHandleMessage = (message, sender, sendResponse) => init.then(() => handleMessage(message, sender, sendResponse).catch(console.error)).catch(console.error);
let jsHandleActionClick = tab => init.then(() => handleActionClick(tab).catch(console.error)).catch(console.error);
let jsHandleTabsUpdated = (tabId, changeInfo, tab) => init.then(() => handleTabsUpdated(tabId, changeInfo, tab).catch(console.error)).catch(console.error);

chrome.runtime.onConnect.addListener(jsHandlePortConnect);
chrome.runtime.onConnectExternal.addListener(jsHandlePortConnect);
chrome.runtime.onMessage.addListener(jsHandleMessage);
chrome.runtime.onMessageExternal.addListener(jsHandleMessage);
chrome.action.onClicked.addListener(jsHandleActionClick);
chrome.tabs.onUpdated.addListener(jsHandleTabsUpdated);
