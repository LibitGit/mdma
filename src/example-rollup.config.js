// This is an example rollup config if you want to try run this yourself.
import rust from "@wasm-tool/rollup-plugin-rust";
import terser from "@rollup/plugin-terser";

export default commandLineArgs => {
    const workspacePath = "extension-workspace/src/";
    const state = new State(commandLineArgs, workspacePath);
    const bundles = [
        state.createBundle("foreground", "foreground/Cargo.toml", {
            outputDir: "extension/wasm/si",
            // extraRustArgs: { cargo: ["--features", "some_feature"], },
        }),
        state.createBundle(
            "foreground",
            "foreground/Cargo.toml",
            {
                outputDir: "extension/wasm/ni",
                extraRustArgs: { cargo: ["--features", "ni"] },
                // extraRustArgs: { cargo: ["--features", "ni, some_other_feature"] },
            }
        ),
        state.createBundle("background", "background/background.js"),
        state.createBundle("popup", "popup/popup.js"),
    ];

    return bundles;
};

class State {
    constructor(commandLineArgs, workspacePath) {
        this.workspacePath = workspacePath;
        this.isWatch = !!process.env.ROLLUP_WATCH;
        this.isProfiling = commandLineArgs.configProfiling;
        this.isProduction = !this.isWatch && !this.isProfiling;
    }

    getRustSettings() {
        let rustSettings = {
            extraArgs: {},
            verbose: true,
            watchPatterns: ["**"],
        };

        if (this.isWatch) {
            rustSettings.extraArgs.wasmBindgen = ["--debug", "--keep-debug"];
            rustSettings.extraArgs.cargo = [
                "--config",
                "env.PROFILING='false'",
            ];
        } else if (this.isProfiling) {
            rustSettings.extraArgs.wasmBindgen = ["--debug", "--keep-debug"];
            rustSettings.extraArgs.cargo = [
                "--config",
                "env.PROFILING='true'",
                "--config",
                "profile.release.trim-paths=true",
            ];
        } else if (this.isProduction) {
            rustSettings.extraArgs.cargo = [
                // "--config",
                // "env.OBFSTR_SEED=''", // <- optional obfuscation seed goes here
                "--config",
                "profile.release.trim-paths=true",
            ];
        }

        return rustSettings;
    }

    createBundle(name, inputPath, options = { extraRustArgs: {} }) {
        const { extraRustArgs = {}, outputDir = "extension/wasm" } = options;
        const { wasmBindgen = [], cargo = [], wasmOpt = this.isProduction || this.isProfiling ? ["-Oz", "--enable-bulk-memory-opt", "--enable-nontrapping-float-to-int"] : [] } = extraRustArgs;
        const rustSettings = this.getRustSettings();

        rustSettings.extraArgs.wasmBindgen = [...(rustSettings.extraArgs.wasmBindgen || []), ...wasmBindgen];
        rustSettings.extraArgs.cargo = [...(rustSettings.extraArgs.cargo || []), ...cargo];
        rustSettings.extraArgs.wasmOpt = [...(rustSettings.extraArgs.wasmOpt || []), ...wasmOpt];

        return {
            input: { [name]: this.workspacePath + inputPath },
            output: {
                dir: (this.isProduction ? "../" : "dev-") + outputDir,
                format: "es",
                // sourcemap: this.isWatch ? "inline" : false,
                // sourcemapBaseUrl: "http://localhost:3000/mdma"
            },
            plugins: [
                rust(rustSettings),
                this.isProduction && terser()
            ]
        };
    }
}