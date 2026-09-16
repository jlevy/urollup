import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { CONFIG_PATH } from "./check-e2e-results.mjs";
import { lintSession, parseBlocks, parseFrontMatter } from "./check-golden-invocations.mjs";
import { VIEWS, goldenPath, renderGolden } from "./new-e2e-golden.mjs";

const ROOT = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const CONFIG = JSON.parse(readFileSync(path.join(ROOT, CONFIG_PATH), "utf8"));

test("a case maps to tests/golden/e2e/<dialect>/<case>.tryscript.md", () => {
  assert.equal(goldenPath(CONFIG, "codex-rollout/archived-rollout"), "tests/golden/e2e/codex-rollout/archived-rollout.tryscript.md");
});

test("the scaffold sandboxes a copy of exactly its case and names only the case's roots", () => {
  const caseId = "claude-project/block-records";
  const text = renderGolden(CONFIG, caseId, { claude: true, codex: false });
  const { entries } = parseFrontMatter(text.split("\n"));
  const sandbox = entries.get("sandbox").value;
  assert.equal(path.resolve(ROOT, path.dirname(goldenPath(CONFIG, caseId)), sandbox), path.join(ROOT, CONFIG.fixturesRoot, "claude-project", "block-records"));
  assert.equal(entries.get("env").map.get("CLAUDE_CONFIG_DIR").value, ".");
  assert.equal(entries.get("env").map.get("CODEX_HOME").value, "$GOLDEN_EMPTY_ROOT");

  const codex = parseFrontMatter(renderGolden(CONFIG, "codex-rollout/x", { claude: false, codex: true }).split("\n")).entries.get("env").map;
  assert.equal(codex.get("CODEX_HOME").value, ".");
  assert.equal(codex.get("CLAUDE_CONFIG_DIR").value, "$GOLDEN_EMPTY_ROOT");
});

test("the scaffold records every view and passes the golden lint once its wildcards are expanded", () => {
  const text = renderGolden(CONFIG, "claude-project/block-records", { claude: true, codex: false });
  assert.deepEqual(
    parseBlocks(text.split("\n")).map((block) => block.commands[0].text),
    VIEWS.map(([, args]) => `urollup ${args}`),
  );
  const findings = lintSession("scaffold.tryscript.md", text);
  assert.equal(findings.length, VIEWS.length);
  assert.ok(findings.every((finding) => /unknown wildcard left in place/.test(finding)), findings.join("\n"));
  assert.deepEqual(lintSession("expanded.tryscript.md", text.replaceAll("\n???\n", "\n{}\n")), []);
});
