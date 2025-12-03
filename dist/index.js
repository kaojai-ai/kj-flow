#!/usr/bin/env node
"use strict";
var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/index.ts
var import_commander4 = require("commander");
var import_dotenv = __toESM(require("dotenv"));

// package.json
var version = "1.0.3";

// src/commands/spec.ts
var import_commander = require("commander");
var import_date_fns = require("date-fns");
var import_path = __toESM(require("path"));
var import_promises = __toESM(require("fs/promises"));
var import_execa2 = require("execa");

// src/utils/git.ts
var import_execa = require("execa");
async function getCurrentBranch() {
  const { stdout } = await (0, import_execa.execa)("git", ["branch", "--show-current"]);
  return stdout.trim();
}
async function getRemoteUrl() {
  try {
    const { stdout } = await (0, import_execa.execa)("git", ["remote", "get-url", "origin"]);
    return stdout.trim();
  } catch {
    return "";
  }
}
async function isGitHubRemote() {
  const remoteUrl = await getRemoteUrl();
  return remoteUrl.includes("github.com");
}
async function getRepoInfo() {
  const remoteUrl = await getRemoteUrl();
  const match = remoteUrl.match(/github\.com[:/]([^/]+)\/([^/.]+)(?:\.git)?/);
  if (match) {
    return { owner: match[1], repo: match[2] };
  }
  return null;
}
async function createBranch(branchName) {
  await (0, import_execa.execa)("git", ["branch", branchName]);
}
async function checkoutBranch(branchName) {
  await (0, import_execa.execa)("git", ["checkout", branchName]);
}

// src/commands/spec.ts
var import_readline = __toESM(require("readline"));
function askQuestion(query) {
  const rl = import_readline.default.createInterface({
    input: process.stdin,
    output: process.stdout
  });
  return new Promise((resolve) => rl.question(query, (ans) => {
    rl.close();
    resolve(ans);
  }));
}
var specCommand = new import_commander.Command("spec").description("Create a new spec file").argument("<ticket-number>", "Ticket number").argument("[summary]", "Summary of the spec").action(async (ticketNumber, summary) => {
  try {
    const currentBranch = await getCurrentBranch();
    const defaultBranches = ["main", "master", "dev"];
    if (defaultBranches.includes(currentBranch)) {
      console.log(`On default branch '${currentBranch}'. Switching to '${ticketNumber}'...`);
      try {
        await createBranch(ticketNumber);
      } catch {
      }
      await checkoutBranch(ticketNumber);
    } else if (currentBranch !== ticketNumber) {
      const answer = await askQuestion(`You are on branch '${currentBranch}'. Do you want to create a new branch '${ticketNumber}' from here? (y/N) `);
      if (answer.toLowerCase() === "y") {
        try {
          await createBranch(ticketNumber);
        } catch {
        }
        await checkoutBranch(ticketNumber);
      }
    }
  } catch (error) {
    console.error("Error handling branch switching:", error);
  }
  const now = /* @__PURE__ */ new Date();
  const year = (0, import_date_fns.format)(now, "yyyy");
  const month = (0, import_date_fns.format)(now, "MM");
  const fileName = summary || ticketNumber;
  const relativePath = import_path.default.join("specs", year, month, ticketNumber);
  const absolutePath = import_path.default.resolve(process.cwd(), relativePath);
  const filePath = import_path.default.join(absolutePath, `${fileName}.md`);
  try {
    await import_promises.default.mkdir(absolutePath, { recursive: true });
    try {
      await import_promises.default.access(filePath);
      console.log(`File already exists: ${filePath}`);
    } catch {
      await import_promises.default.writeFile(filePath, `---
ticket_number: ${ticketNumber}
---

${summary || ""}
`);
      console.log(`Created spec file: ${filePath}`);
    }
    const ide = process.env.KJ_IDE || "antigravity";
    console.log(`Opening in ${ide}...`);
    try {
      await (0, import_execa2.execa)(ide, [filePath], { stdio: "inherit" });
    } catch (error) {
      console.error(`Failed to open with ${ide}. Please check if it is installed or set KJ_IDE env var.`);
      console.error(error);
    }
  } catch (error) {
    console.error("Error creating spec:", error);
    process.exit(1);
  }
});

// src/commands/pr.ts
var import_commander2 = require("commander");
var import_execa3 = require("execa");
var import_promises2 = __toESM(require("fs/promises"));
var import_path2 = __toESM(require("path"));
async function findSpecFile(ticketNumber) {
  const specsDir = import_path2.default.join(process.cwd(), "specs");
  try {
    const years = await import_promises2.default.readdir(specsDir);
    for (const year of years) {
      const yearPath = import_path2.default.join(specsDir, year);
      const yearStat = await import_promises2.default.stat(yearPath);
      if (!yearStat.isDirectory()) continue;
      const months = await import_promises2.default.readdir(yearPath);
      for (const month of months) {
        const monthPath = import_path2.default.join(yearPath, month);
        const monthStat = await import_promises2.default.stat(monthPath);
        if (!monthStat.isDirectory()) continue;
        const tickets = await import_promises2.default.readdir(monthPath);
        if (tickets.includes(ticketNumber)) {
          const ticketPath = import_path2.default.join(monthPath, ticketNumber);
          const files = await import_promises2.default.readdir(ticketPath);
          const mdFile = files.find((f) => f.endsWith(".md"));
          if (mdFile) {
            return import_path2.default.join(ticketPath, mdFile);
          }
        }
      }
    }
  } catch (e) {
  }
  return null;
}
async function getArtifactsMetadata(ticketNumber) {
  const specsDir = import_path2.default.join(process.cwd(), "specs");
  try {
    const years = await import_promises2.default.readdir(specsDir);
    for (const year of years) {
      const yearPath = import_path2.default.join(specsDir, year);
      const yearStat = await import_promises2.default.stat(yearPath);
      if (!yearStat.isDirectory()) continue;
      const months = await import_promises2.default.readdir(yearPath);
      for (const month of months) {
        const monthPath = import_path2.default.join(yearPath, month);
        const monthStat = await import_promises2.default.stat(monthPath);
        if (!monthStat.isDirectory()) continue;
        const tickets = await import_promises2.default.readdir(monthPath);
        if (tickets.includes(ticketNumber)) {
          const metadataPath = import_path2.default.join(monthPath, ticketNumber, "artifacts", "metadata.json");
          try {
            const content = await import_promises2.default.readFile(metadataPath, "utf-8");
            return JSON.parse(content);
          } catch (e) {
            return null;
          }
        }
      }
    }
  } catch (e) {
  }
  return null;
}
var prCommand = new import_commander2.Command("pr").description("Pull Request management");
prCommand.command("create").description("Create a Pull Request").argument("[ticket-number]", "Ticket number (defaults to current branch)").option("--dry-run", "Dry run mode").action(async (ticketNumber, options) => {
  try {
    const currentBranch = await getCurrentBranch();
    const ticket = ticketNumber || currentBranch;
    if (!await isGitHubRemote()) {
      console.error("Error: Not a GitHub repository or remote is not configured.");
      process.exit(1);
    }
    console.log(`Creating PR for ticket: ${ticket}...`);
    const specFile = await findSpecFile(ticket);
    let specContent = "";
    if (specFile) {
      console.log(`Found spec file: ${specFile}`);
      const content = await import_promises2.default.readFile(specFile, "utf-8");
      specContent = content.replace(/^---\n[\s\S]*?\n---\n/, "").trim();
    } else {
      console.warn(`Warning: Spec file not found for ticket ${ticket}.`);
    }
    const metadata = await getArtifactsMetadata(ticket);
    if (!metadata) {
      console.error(`Error: metadata.json not found for ticket ${ticket}. Please ensure specs/YYYY/MM/${ticket}/artifacts/metadata.json exists.`);
      process.exit(1);
    }
    const { pr_title, pr_summary } = metadata;
    const title = pr_title || `feat: ${ticket}`;
    const repoInfo = await getRepoInfo();
    const repoUrlPrefix = repoInfo ? `https://github.com/${repoInfo.owner}/${repoInfo.repo}/blob/${ticket}` : "";
    const prBody = `# AI Summary
${pr_summary}

# User Prompt
${specContent}

# Artifacts
- [implementation_plan.md](${repoUrlPrefix}/specs/2025/12/${ticket}/artifacts/implementation_plan.md)
- [walkthrough.md](${repoUrlPrefix}/specs/2025/12/${ticket}/artifacts/walkthrough.md)`;
    if (options.dryRun) {
      console.log("--- Dry Run ---");
      console.log(`Title: ${title}`);
      console.log(`Body:
${prBody}`);
      console.log(`Head: ${currentBranch}`);
      console.log("Command: gh pr create --title ... --body ... --head ...");
      return;
    }
    console.log("Running gh pr create...");
    await (0, import_execa3.execa)("gh", ["pr", "create", "--title", title, "--body", prBody, "--head", currentBranch], { stdio: "inherit" });
  } catch (error) {
    console.error("Error creating PR:", error);
    process.exit(1);
  }
});
prCommand.command("merge").description("Merge a Pull Request").argument("[ticket-number]", "Ticket number (defaults to current branch)").action(async (ticketNumber) => {
  try {
    console.log("Running gh pr merge...");
    await (0, import_execa3.execa)("gh", ["pr", "merge", "--auto", "--merge"], { stdio: "inherit" });
  } catch (error) {
    console.error("Error merging PR:", error);
    process.exit(1);
  }
});

// src/commands/ping.ts
var import_commander3 = require("commander");
var pingCommand = new import_commander3.Command("ping").description("Ping the CLI to check if it is working").action(() => {
  console.log("pong");
});

// src/index.ts
import_dotenv.default.config();
var program = new import_commander4.Command();
program.name("kj").description("Kaojai Flow CLI").version(version);
program.addCommand(specCommand);
program.addCommand(prCommand);
program.addCommand(pingCommand);
program.parse(process.argv);
