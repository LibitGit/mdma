(async () => {
    console.log("%cMDMA FOREGROUND INJECTED", 'color:gold', performance.now());

    // DEBUG
    if (window.mdma?.foreground) {
        return;
    }
    window.mdma ||= {};
    window.mdma.foreground = true;
    // DEBUG END

    //TODO: Implement actual caching behavior
    let cacheToken = window.localStorage.getItem('mdma_cache');
    if (!cacheToken) {
        cacheToken = Math.random().toString(36).substring(2);
        window.localStorage.setItem('mdma_cache', cacheToken);
    }

    // DEBUG
    cacheToken = Date.now();

    let originalEngine = undefined;
    let originalCommunication = undefined;
    let originalInit = undefined;
    let originalInitCalled = false;
    let originalInitThis = null;

    Object.defineProperty(window, 'Engine', {
        get: () => originalEngine,
        set: onEngineSet
    });

    function onEngineSet(Engine) {
        Object.defineProperty(Engine, 'communication', {
            get: () => originalCommunication,
            set: onCommunicationSet
        });

        originalEngine = Engine;
    }

    function onCommunicationSet(communication) {
        instantiateWebAssembly();

        originalInit = communication.startCallInitAfterRecivedAddons;
        communication.startCallInitAfterRecivedAddons = () => {
            // DEBUG
            console.log("%cMARGONEM CALLED INIT", 'color:gold', performance.now());

            originalInitCalled = true;
            originalInitThis = this;
        }
        originalCommunication = communication;

        return true;
    }

    async function instantiateWebAssembly() {
        const {default: init, init_mdma} = await import(`http://localhost:3000/mdma/foreground.js?c=${cacheToken}`);
        console.log("%cMDMA FOREGROUND FETCHED", 'color:gold', performance.now());

        // DEBUG
        const start = performance.now();

        await init(`http://localhost:3000/mdma/foreground_bg.wasm?c=${cacheToken}`);

        // DEBUG
        console.log(`%cMDMA FOREGROUND INIT TOOK ${performance.now() - start}ms`, 'color:gold');

        await init_mdma();

        const interval = setInterval(() => {
            if (!originalInitCalled) {
                return;
            }

            // DEBUG
            console.log("%cMDMA CALLING MARGONEM INIT", 'color:gold', performance.now());

            clearInterval(interval);
            originalInit.call(originalInitThis);
            window.Engine.communication.startCallInitAfterRecivedAddons = originalInit;
        });
    }
})();
