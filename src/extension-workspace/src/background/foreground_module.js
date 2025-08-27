(function () {
  const OriginalWebSocket = window.WebSocket;
  const INTERFACE = document.cookie.match(/interface=(\w+)/)?.[1];
  const TLD = window.location.origin.split('.').pop();
  const log = (...args) => console.log(
    "%c MDMA %c %c JavaScript ",
    "background: #8CAAEE; color: black; font-weight: bold; border-radius: 5px;",
    "",
    "background: #F0DB4F; color: black; font-weight: bold; border-radius: 5px;",
    ...args
  );
  const error = (...args) => console.error(
    "%c MDMA %c %c JavaScript ",
    "background: #8CAAEE; color: black; font-weight: bold; border-radius: 5px;",
    "",
    "background: #F0DB4F; color: black; font-weight: bold; border-radius: 5px;",
    ...args
  );

  switch (window.location.href) {
    case "https://www.margonem.pl/":
    case "https://www.margonem.com/":
    case "https://commons.margonem.pl/":
    case "https://dev-commons.margonem.pl/":
    case "https://commons.margonem.com/":
    case "https://dev-commons.margonem.com/": return;
    default:
      log("Foreground module injected!")
  }

  if (INTERFACE !== 'ni' && INTERFACE !== 'si') {
    return error("Stopping init due to unrecognised game interface.", INTERFACE);
  }
  if (TLD !== 'pl' && TLD !== 'com') {
    return error("Stopping init due to unrecognised top level domain.", TLD);
  }

  // TODO: Look below.
  // const JS_MODULE_URL = "http://localhost:3000/mdma/foregroundNi.js";
  // In your content script
  // const EXTENSION_VERSION = "__VERSION__"; // This will be replaced during build

  // In your build script (webpack, rollup, etc.)
  // replace({
  //   '__VERSION__': JSON.parse(fs.readFileSync('manifest.json')).version
  // })
  const JS_MODULE_URL = `./wasm/${INTERFACE}/foreground.js`;
  // const JS_VERSION = "0.15.0";

  // TODO: Implement actual caching behavior
  // let cacheToken = Date.now();
  let originalInitWebSocket = null;
  let originalInitCalled = false;
  let initProcedureFailed = false;
  let observeOnmessage = null;

  instantiateWebAssembly();

  // Has to be defined before the game's WebSocket proxy.
  window.WebSocket = function WebSocketHook(url, protocols) {
    const socket = new OriginalWebSocket(url, protocols);

    if (url.endsWith(".margonem.pl/ws-engine") || url.endsWith(".margonem.com/ws-engine")) {
      function setterCallback(communication) {
        observeOnmessage(this, communication);
      };

      const properties = Object.getOwnPropertyDescriptor(OriginalWebSocket.prototype, 'onmessage');

      try {
        defineProperty(socket, 'onmessage', { setterCallback, properties });
      } catch (e) {
        initProcedureFailed = true;
        error(e.message);
      }
    }

    return socket;
  };

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
   * Defines a property on an object and binds callbacks to its getter and setter.
   *
   * @param {Object} object - The object on which to define the property.
   * @param {string} key - The key of the property to define on the object.
   * @param {Object} [options={}] - Options for configuring the property.
   * @param {setterCallback} [options.setterCallback=() => {}] - A callback function to execute after the setter gets executed.
   * @param {getterCallback} [options.getterCallback=() => {}] - A callback function to execute before the getter gets executed.
   * @param {boolean} [options.configurable=true] - Whether the property can be redefined or deleted.
   * @param {boolean} [options.enumerable=true] - Whether the property shows up in enumeration of the object.
   * @param {Object} [options.properties=Object.getOwnPropertyDescriptor(object, key)] - Additional property descriptors to apply.
   * @throws {Error} Throws an error if the first parameter is not an instance of `Object` or `Function`.
   * @throws {Error} Throws an error if the second parameter is not a string.
   * @throws {Error} Throws an error if the target object property is not configurable.
   */
  function defineProperty(object, key, options = {}) {
    if (typeof object !== "object" && typeof object !== "function") throw new Error("`object` has to be an instance of `Object` or `Function`.");
    if (typeof key !== "string") throw new Error("`key` has to be an instance of `String`");

    const {
      setterCallback = () => { },
      getterCallback = () => { },
      configurable = true,
      enumerable = true,
      properties = Object.getOwnPropertyDescriptor(object, key),
    } = options;

    if (properties && !properties.configurable) throw new Error(`Property: ${key} is not configurable!`);

    let oldGetter, oldSetter;
    if (properties) {
      oldGetter = properties.get;
      oldSetter = properties.set;
    }

    let value = object[key];

    Object.defineProperty(object, key, {
      get() {
        getterCallback();

        return oldGetter ? oldGetter.call(this) : value;
      },
      set(new_value) {
        if (typeof new_value === 'object') {
          setterCallback.call(this, new_value);
        } else {
          const new_value_wrapper = { [key]: new_value };
          setterCallback.call(this, new_value_wrapper);
          new_value = new_value_wrapper[key]
        }

        value = new_value;

        if (oldSetter) oldSetter.call(this, new_value);

        return true;
      },
      configurable,
      enumerable,
    });
  }

  function onCommunicationSet(communication) {
    originalInitWebSocket = communication.initWebSocket;
    communication.initWebSocket = function () {
      log("Game called init.");

      originalInitCalled = true;
    };
  }

  try {
    hook()
  } catch (e) {
    error(e.message);
    // window.location.reload()
  }

  function hook() {
    if ((typeof Object.getOwnPropertyDescriptor(OriginalWebSocket.prototype, 'onmessage')) !== 'object') {
      throw new Error("Foreground module injected after `WebSocket` was proxied.");
    }

    switch (INTERFACE) {
      case "si":
        if (typeof window.initWebSocket === 'undefined') {
          defineProperty(window, 'initWebSocket', { setterCallback: onCommunicationSet });
          return;
        }

        break;
      case "ni":
        if (typeof window.Engine === "undefined") {
          defineProperty(window, 'Engine', { setterCallback: function (engine) { defineProperty(engine, 'communication', { setterCallback: onCommunicationSet }) } });
          return;
        }
        if (typeof window.Engine.communication === "undefined") {
          defineProperty(window.Engine, 'communication', { setterCallback: onCommunicationSet });
          return;
        }

        break;
    }

    throw new Error("Foreground module injected after communication module was set.");
  }

  function notifyFailedInit() {
    if (typeof window.message === 'function') window.message("[MDMA::JS] Błąd podczas wczytywania zestawu!");
    log("Stopping init due to previous error...");
  }

  async function instantiateWebAssembly() {
    log("Instantiating WASM module...")
    const module = await import(JS_MODULE_URL);
    observeOnmessage = module.observeOnmessage;

    if (initProcedureFailed) {
      notifyFailedInit();
      return;
    }
    if (originalInitCalled) {
      module.main(originalInitWebSocket).catch(console.error);
      return;
    }

    log("Waiting for game init...");

    let timedOut = false;
    const interval = setInterval(() => {
      if (initProcedureFailed) {
        clearInterval(interval);
        notifyFailedInit();
        return;
      }
      if (originalInitCalled) {
        clearInterval(interval);
        module.main(originalInitWebSocket).catch(console.error);
        return;
      }
      if (timedOut) {
        clearInterval(interval);
        log("Game init not called in over 60 seconds, assuming unhandled environment...");
      }
    }, 50);
    setTimeout(() => timedOut = true, 60_000);
  }
})()