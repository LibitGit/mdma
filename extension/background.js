import init from "http://localhost:3000/mdma/background.js";
init(`http://localhost:3000/mdma/background_bg.wasm?c=${Date.now()}`).then(() => {});