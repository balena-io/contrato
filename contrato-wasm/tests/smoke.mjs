// Smoke test for the contrato-wasm crate.
//
// Run after `wasm-pack build contrato-wasm --target nodejs --out-dir pkg`:
//
//     node contrato-wasm/tests/smoke.mjs
//
// Exits with code 0 on success, 1 on the first failed assertion. Each step
// prints a short `[ok]` or `[fail]` line so the log reads cleanly when
// scanning CI output or local runs.

import { Contract, Universe } from "../pkg/contrato_wasm.js";

let failures = 0;

function check(label, condition, detail) {
    if (condition) {
        console.log(`[ok]   ${label}`);
    } else {
        console.log(`[fail] ${label}${detail ? `: ${detail}` : ""}`);
        failures += 1;
    }
}

function eq(label, actual, expected) {
    const a = JSON.stringify(actual);
    const e = JSON.stringify(expected);
    check(label, a === e, `expected ${e}, got ${a}`);
}

// ── 1. Construct a Contract from plain JSON ───────────────────────────────

const raspberryPiJson = {
    type: "hw.device-type",
    slug: "raspberry-pi",
    name: "Raspberry Pi",
    version: "1",
    data: { arch: "armv6" },
};
const raspberryPi = new Contract(raspberryPiJson);

eq("raspberry-pi getType", raspberryPi.getType(), "hw.device-type");
eq("raspberry-pi getSlug", raspberryPi.getSlug(), "raspberry-pi");
eq("raspberry-pi getVersion", raspberryPi.getVersion(), "1");
eq(
    "raspberry-pi getReferenceString",
    raspberryPi.getReferenceString(),
    "raspberry-pi@1",
);
check(
    "raspberry-pi hash is non-empty string",
    typeof raspberryPi.hash() === "string" && raspberryPi.hash().length > 0,
);

// toJSON round-trip preserves the input.
const roundTripped = raspberryPi.toJSON();
eq("raspberry-pi toJSON type", roundTripped.type, "hw.device-type");
eq("raspberry-pi toJSON slug", roundTripped.slug, "raspberry-pi");
eq("raspberry-pi toJSON data.arch", roundTripped.data.arch, "armv6");

// ── 2. Universe with children and matcher-based search ───────────────────

const universe = new Universe();
// Universe only exposes its constructor in Phase 13; the Phase 14 TS
// wrapper re-wires Contract's addChildren onto a `class Universe extends
// Contract` shim. For the smoke test we skip Universe's children and
// exercise Contract directly.

const parent = new Contract({
    type: "meta.universe",
    slug: "test-universe",
});
parent.addChildren([
    new Contract({ type: "hw.device-type", slug: "raspberry-pi", version: "1" }),
    new Contract({ type: "hw.device-type", slug: "raspberry-pi2", version: "2" }),
    new Contract({ type: "arch.sw", slug: "armv6" }),
]);

eq("universe get_children length", parent.getChildren().length, 3);
eq(
    "universe get_children_by_type(hw.device-type) length",
    parent.getChildrenByType("hw.device-type").length,
    2,
);
const childrenTypes = parent.getChildrenTypes();
check(
    "universe children types contain hw.device-type and arch.sw",
    childrenTypes.includes("hw.device-type") && childrenTypes.includes("arch.sw"),
);

// findChildren accepts a plain JS matcher object.
const foundPi2 = parent.findChildren({
    type: "hw.device-type",
    slug: "raspberry-pi2",
});
eq("findChildren(pi2) length", foundPi2.length, 1);
eq("findChildren(pi2) slug", foundPi2[0].getSlug(), "raspberry-pi2");

const foundAny = parent.findChildren({ type: "hw.device-type" });
eq("findChildren(hw.device-type only) length", foundAny.length, 2);

// ── 3. Requirement satisfaction ───────────────────────────────────────────

const dependent = new Contract({
    type: "sw.stack",
    slug: "nodejs",
    requires: [{ type: "hw.device-type", slug: "raspberry-pi" }],
});
const requirementTypes = dependent.getRequirementTypes();
check(
    "dependent getRequirementTypes contains hw.device-type",
    requirementTypes.includes("hw.device-type"),
);

const depMatchers = dependent.getRequirementMatchersForType("hw.device-type");
eq("dependent getRequirementMatchersForType length", depMatchers.length, 1);
eq(
    "dependent getRequirementMatchersForType[0] type",
    depMatchers[0].type,
    "hw.device-type",
);
eq(
    "dependent getRequirementMatchersForType[0] slug",
    depMatchers[0].slug,
    "raspberry-pi",
);

// A matcher returned from getRequirementMatchersForType is directly
// re-usable as a findChildren argument — Phase 14's TS port of
// getReferencedContracts walks the requirements index via exactly
// this path.
const refetch = parent.findChildren(depMatchers[0]);
eq(
    "parent.findChildren(requirementMatcher) length",
    refetch.length,
    1,
);

check(
    "parent satisfies dependent (raspberry-pi present)",
    parent.satisfiesChildContract(dependent),
);

const unsatisfied = new Contract({
    type: "sw.stack",
    slug: "nodejs",
    requires: [{ type: "hw.device-type", slug: "banana-pi" }],
});
check(
    "parent does NOT satisfy banana-pi requirement",
    !parent.satisfiesChildContract(unsatisfied),
);

const unsatisfiedReqs = parent.getNotSatisfiedChildRequirements(unsatisfied);
check(
    "getNotSatisfiedChildRequirements returns one entry",
    Array.isArray(unsatisfiedReqs) && unsatisfiedReqs.length === 1,
    `got ${JSON.stringify(unsatisfiedReqs)}`,
);

// ── 4. Contract.build: variants + aliases ────────────────────────────────

const variants = Contract.build({
    type: "sw.os",
    slug: "debian",
    aliases: ["deb"],
    variants: [
        { version: "wheezy" },
        { version: "jessie" },
    ],
});
eq(
    "Contract.build produces 4 contracts (2 variants × 2 aliases each)",
    variants.length,
    4,
);

// ── 5. Contract.isEqual ──────────────────────────────────────────────────

const a = new Contract({ type: "hw.device-type", slug: "raspberry-pi" });
const b = new Contract({ type: "hw.device-type", slug: "raspberry-pi" });
const c = new Contract({ type: "hw.device-type", slug: "raspberry-pi2" });
check("Contract.isEqual(a, b) is true", Contract.isEqual(a, b));
check("Contract.isEqual(a, c) is false", !Contract.isEqual(a, c));

// ── summary ───────────────────────────────────────────────────────────────

if (failures > 0) {
    console.error(`\n${failures} check(s) failed`);
    process.exit(1);
}
console.log("\nall smoke checks passed");
