// A dependency-free PostCSS config: one inline plugin that shows what the child process
// sees (HUGO_ENVIRONMENT, the cwd, NODE_ENV which Hugo filters out) and rewrites a declaration.
module.exports = {
  plugins: [
    {
      postcssPlugin: "t16",
      Declaration: {
        color: (decl) => {
          if (decl.value === "red") decl.value = "#f00";
        },
      },
      OnceExit(root) {
        root.append({
          text: "env=" + process.env.HUGO_ENVIRONMENT + " node_env=" + (process.env.NODE_ENV || "-") +
            " cwd=" + require("path").basename(process.cwd()) +
            " files=" + Object.keys(process.env).filter((k) => k.startsWith("HUGO_FILE_")).sort().join(","),
        });
      },
    },
  ],
};
