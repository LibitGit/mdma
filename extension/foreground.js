(async () => {
    const {default: init} = await import("https://libit.ovh/mdma/foreground.js");
    await init();
})();