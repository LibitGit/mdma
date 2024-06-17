(async () => {
    const {default: init} = await import(`https://libit.ovh/mdma/foreground.js?c=${Date.now()}`);
    await init(`https://libit.ovh/mdma/foreground_bg.wasm?c=${Date.now()}`);
})();