#!/usr/bin/env node

const { runXiaomiMiMoTui } = require("../scripts/run");

runXiaomiMiMoTui().catch((error) => {
  console.error("Failed to start xiaomimimo-tui:", error.message);
  process.exit(1);
});
