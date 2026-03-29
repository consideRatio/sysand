const path = require("path");
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");

module.exports = {
  entry: ["./src/sysand.js"],
  output: {
    path: path.resolve(__dirname, "browser_dist"),
    filename: "bundle.js",
    library: {
      type: "module",
    },
  },
  plugins: [
    new WasmPackPlugin({
      crateDirectory: path.resolve(__dirname),
      outDir: path.resolve(__dirname, "browser_pkg"),
    }),
  ],
  resolve: {
    modules: ["node_modules"],
    extensions: ["*", ".js", ".jsx", ".tsx", ".ts"],
    alias: {
      "sysand-wasm": path.resolve(__dirname, "browser_pkg"),
    },
  },
  mode: "development",
  experiments: {
    asyncWebAssembly: true,
    outputModule: true,
  },
};
