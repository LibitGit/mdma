import { module, init } from "./Cargo.toml?custom";

let exports = new Promise(async res => { res(await init({ module: module })) });

let handlePortConnect = async (port) =>
    (await exports).handlePortConnect(port);

let handleMessage = async (message, sender, sendResponse) =>
    (await exports).handleMessage(message, sender, sendResponse);

let handleActionClick = async (tab) =>
    (await exports).handleActionClick(tab);

// let handleTabsUpdated = async (tabId, changeInfo, tab) =>
//     (await exports).handleTabsUpdated(tabId, changeInfo, tab);

let handleOnCommitted = async (details) =>
    (await exports).handleOnCommitted(details);

chrome.runtime.onConnect.addListener(handlePortConnect);
chrome.runtime.onConnectExternal.addListener(handlePortConnect);
chrome.runtime.onMessage.addListener(handleMessage);
chrome.runtime.onMessageExternal.addListener(handleMessage);
chrome.action.onClicked.addListener(handleActionClick);
// chrome.webNavigation.onCommitted.addListener(handleOnCommitted)
// chrome.tabs.onUpdated.addListener(handleTabsUpdated);
