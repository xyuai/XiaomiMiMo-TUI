#!/usr/bin/env node

const { runXiaomiMiMo } = require("../scripts/run");

runXiaomiMiMo().catch((error) => {
  console.error("Failed to start xiaomimimo:", error.message);
  process.exit(1);
});
