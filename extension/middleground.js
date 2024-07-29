(async () => {
    // DEBUG
    if (window.mdma) {
        return;
    }
    window.mdma ||= {};
    window.mdma.middleground = true;

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

    const {default: init} = await import(`https://libit.ovh/mdma/middleground.js?c=${cacheToken}`);
    console.log("MDMA MIDDLEGROUND FETCHED", performance.now());

    let start = performance.now();
    await init(`https://libit.ovh/mdma/middleground_bg.wasm?c=${cacheToken}`);
    console.log(`Middleground init took ${performance.now() - start}ms`);
})();
