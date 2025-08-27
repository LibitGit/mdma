import { module, init } from "./Cargo.toml?custom";

let exports = new Promise(async res => { res(await init({ module: module })) });

let handleMessage = async (message, sender, sendResponse) =>
    (await exports).handleMessage(message, sender, sendResponse);

chrome.runtime.onMessage.addListener(handleMessage);
