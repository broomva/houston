import test from "node:test";
import assert from "node:assert/strict";
import {
  applySlashSkillSelection,
  filterSlashSkillOptions,
  getSlashSkillQuery,
} from "../src/slash-skills.ts";

const skills = [
  {
    id: "agent:/a",
    name: "Batch",
    description: "Research and plan a large change",
    sourceLabel: "Agent",
  },
  {
    id: "global:/b",
    name: "Browser",
    description: "Headless browser automation",
    sourceLabel: "Claude",
  },
];

test("detects slash skill query at the current composer token", () => {
  assert.equal(getSlashSkillQuery("/b"), "b");
  assert.equal(getSlashSkillQuery("hello\n/bro"), "bro");
  assert.equal(getSlashSkillQuery("hello /b"), null);
  assert.equal(getSlashSkillQuery("/browser now"), null);
});

test("removes the slash token after selection", () => {
  assert.equal(applySlashSkillSelection("/b"), "");
  assert.equal(applySlashSkillSelection("hello\n/bro"), "hello\n");
});

test("filters by name, description, or source label", () => {
  assert.deepEqual(
    filterSlashSkillOptions(skills, "b").map((skill) => skill.name),
    ["Batch", "Browser"],
  );
  assert.deepEqual(
    filterSlashSkillOptions(skills, "headless").map((skill) => skill.name),
    ["Browser"],
  );
  assert.deepEqual(
    filterSlashSkillOptions(skills, "claude").map((skill) => skill.name),
    ["Browser"],
  );
});
