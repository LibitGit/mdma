
(async () => {
    console.log("%cMDMA FOREGROUND INJECTED", 'color:gold', performance.now());

    //TODO: Implement actual caching behavior
    let cacheToken = window.localStorage.getItem('mdma_cache');
    if (!cacheToken) {
        cacheToken = Math.random().toString(36).substring(2);
        window.localStorage.setItem('mdma_cache', cacheToken);
    }

    // DEBUG
    cacheToken = Date.now();

    instantiateWebAssembly().then(() => console.log("%cMDMA FOREGROUND INSTANTIATED", 'color:gold', performance.now()));

    let originalInit = undefined;
    let originalInitCalled = false;
    let originalInitThis = null;
    let alreadyInstantiated = false;

    if (alreadyInstantiated) return;

    /**
     * A callback function that is invoked after the property setter is called.
     *
     * @callback setterCallback
     * @param {*} value - The new value being set on the property.
     */

    /**
     * A callback function that is invoked when the property getter is called.
     *
     * @callback getterCallback
     * @param {*} value - The new value being set on the property.
     */

    /**
     * Defines a property on an object and binds a callback to its setter.
     *
     * @param {Object} object - The object on which to define the property.
     * @param {string} key - The key of the property to define on the object.
     * @param {setterCallback} setterCallback - A callback function to execute after the setter gets executed.
     * @param {getterCallback} getterCallback - A callback function to execute before the getter gets executed.
     * @param {boolean} configurable - Whether the property can be redefined or deleted.
     * @param {boolean} enumerable - Whether the property shows up in enumeration of the object.
     * @throws {Error} Throws an error if the first parameter is not an object.
     * @throws {Error} Throws an error if the second parameter is not a string.
     * @throws {Error} Throws an error if the target object property is not configurable.
     */
    function defineProperty(object, key, setterCallback = () => {
    }, getterCallback = () => {
    }, configurable = true, enumerable = true) {
        if (typeof object !== "object") throw new Error("typeof object !== \"object\"");
        if (typeof key !== "string") throw new Error("typeof key !== \"string\"");

        const properties = Object.getOwnPropertyDescriptor(object, key);

        if (properties && !properties.configurable) throw new Error("Property is not configurable!");

        let oldGetter, oldSetter;
        if (properties) {
            oldGetter = properties.get;
            oldSetter = properties.set;
        }

        let value = object[key];
        Object.defineProperty(object, key, {
            get() {
                getterCallback();

                return oldGetter ? oldGetter() : value;
            }, set(new_value) {
                value = new_value;

                setterCallback(value);
                if (oldSetter) oldSetter(value);
            }, configurable, enumerable,
        });
    }

    try {
        if (typeof window.Engine === "undefined") return defineProperty(window, 'Engine', onEngineSet);
        if (typeof window.Engine.communication === "undefined") return defineProperty(window.Engine, 'communication', onCommunicationSet);

        console.warn("%c[MDMA] Foreground script injected after communication set, not all functionalities might work properly until map change or page reload", 'color:red');
        overrideInit(window.Engine.communication);
    } catch (e) {
        console.error(`%c${e.message}`, "color:red");
    }

    function onEngineSet(engine) {
        if (alreadyInstantiated) return console.log("%cMDMA INSTANTIATED BEFORE ENGINE SET", 'color:gold', performance.now());

        defineProperty(engine, 'communication', onCommunicationSet);
    }

    function onCommunicationSet(communication) {
        if (alreadyInstantiated) return console.log("%cMDMA INSTANTIATED BEFORE COMMUNICATION SET", 'color:gold', performance.now());

        overrideInit(communication);
    }

    function overrideInit(communication) {
        originalInit = communication.startCallInitAfterRecivedAddons;
        communication.startCallInitAfterRecivedAddons = () => {
            // DEBUG
            console.log("%cMARGONEM CALLED INIT", 'color:gold', performance.now());

            originalInitCalled = true;
            originalInitThis = this;
        }
    }

    async function instantiateWebAssembly() {
        const {default: init} = await import(`https://libit.ovh/mdma/foreground.js?c=${cacheToken}`);
        console.log("%cMDMA FOREGROUND FETCHED", "color:gold", performance.now());

        // DEBUG
        const start = performance.now();

        await init({module_or_path: `https://libit.ovh/mdma/foreground_bg.wasm?c=${cacheToken}`});

        // DEBUG
        console.log(`%cMDMA FOREGROUND INIT TOOK ${performance.now() - start}ms`, 'color:gold');

        if (!window?.Engine?.communication) {
            alreadyInstantiated = true;
            return console.log("alreadyInstantiated = true");
        }

        const interval = setInterval(() => {
            if (!originalInitCalled) {
                return console.log("%cMDMA WAITING FOR MARGONEM INIT", 'color:gold', performance.now());
            }

            // DEBUG
            console.log("%cMDMA CALLING MARGONEM INIT", 'color:gold', performance.now());

            clearInterval(interval);
            originalInit.call(originalInitThis);
            window.Engine.communication.startCallInitAfterRecivedAddons = originalInit;
        }, 100);
    }
})();

