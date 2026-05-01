const { spawnSync } = require("child_process");
const { getBinaryPath } = require("./install");

async function run(binaryName) {
  const binaryPath = await getBinaryPath(binaryName);
  const result = spawnSync(binaryPath, process.argv.slice(2), {
    stdio: "inherit",
  });
  if (result.error) {
    throw result.error;
  }
  process.exit(result.status ?? 1);
}

async function runXiaomiMiMo() {
  await run("xiaomimimo");
}

async function runXiaomiMiMoTui() {
  await run("xiaomimimo-tui");
}

module.exports = {
  run,
  runXiaomiMiMo,
  runXiaomiMiMoTui,
};

if (require.main === module) {
  const command = process.argv[1] || "";
  if (command.includes("tui")) {
    runXiaomiMiMoTui();
  } else {
    runXiaomiMiMo();
  }
}
