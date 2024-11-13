(async () => {
    console.log("MDMA MIDDLEGROUND INJECTED", performance.now());
    // DEBUG END

    //TODO: Implement actual caching behaviour
    let cacheToken = window.localStorage.getItem('mdma_cache');
    if (!cacheToken) {
        cacheToken = Math.random().toString(36).substring(2);
        window.localStorage.setItem('mdma_cache', cacheToken);
    }

    // DEBUG
    cacheToken = Date.now();
    let port = chrome.runtime.connect({name: "middleground"});

    port.onDisconnect.addListener(console.log);
    
    console.log("MIDDLEGROUND CONNECTED TO PORT")
    port.onMessage.addListener(function (msg) {
        let handle = () => {
            window.dispatchEvent(new CustomEvent("mdma_id", {detail: {value: msg}}))
            
            console.log("mdma_id EVENT DISPATCHED")
            window.removeEventListener("mdma_foreground_init", handle)
        }
        window.addEventListener("mdma_foreground_init", handle)
    })

    // const {default: init} = await import(`https://libit.ovh/mdma/middleground.js?c=${cacheToken}`);
    // console.log("MDMA MIDDLEGROUND FETCHED", performance.now());
    //
    // let start = performance.now();
    // await init(`https://libit.ovh/mdma/middleground_bg.wasm?c=${cacheToken}`);
    // console.log(`Middleground init took ${performance.now() - start}ms`);
})();
