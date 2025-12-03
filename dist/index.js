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
var version = "1.0.0";

// src/commands/spec.ts
var import_commander = require("commander");
var import_date_fns = require("date-fns");
var import_path = __toESM(require("path"));
var import_promises = __toESM(require("fs/promises"));
var import_execa = require("execa");
var specCommand = new import_commander.Command("spec").description("Create a new spec file").argument("<ticket-number>", "Ticket number").argument("[summary]", "Summary of the spec").action(async (ticketNumber, summary) => {
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
      await import_promises.default.writeFile(filePath, `# ${ticketNumber}: ${summary || ""}

`);
      console.log(`Created spec file: ${filePath}`);
    }
    const ide = process.env.KJ_IDE || "antigravity";
    console.log(`Opening in ${ide}...`);
    try {
      await (0, import_execa.execa)(ide, [filePath], { stdio: "inherit" });
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

// src/utils/git.ts
var import_execa2 = require("execa");
async function getCurrentBranch() {
  const { stdout } = await (0, import_execa2.execa)("git", ["branch", "--show-current"]);
  return stdout.trim();
}
async function getRemoteUrl() {
  try {
    const { stdout } = await (0, import_execa2.execa)("git", ["remote", "get-url", "origin"]);
    return stdout.trim();
  } catch {
    return "";
  }
}
async function isGitHubRemote() {
  const remoteUrl = await getRemoteUrl();
  return remoteUrl.includes("github.com");
}
async function getDiff(baseBranch = "main") {
  try {
    const { stdout } = await (0, import_execa2.execa)("git", ["diff", baseBranch]);
    return stdout;
  } catch {
    return "";
  }
}

// src/utils/openai.ts
var import_openai = __toESM(require("openai"));
async function generatePrDetails(diff, specContent) {
  const apiKey = process.env.OPENAI_API_KEY;
  if (!apiKey) {
    throw new Error("OPENAI_API_KEY is not set");
  }
  const openai = new import_openai.default({ apiKey });
  const prompt = `
You are a helpful assistant that generates Pull Request titles and summaries.
Based on the following code diff and the original spec, please generate a concise PR title and a detailed summary.

Spec:
${specContent}

Diff:
${diff.substring(0, 1e4)} // Truncate diff to avoid token limits if necessary

Output format (JSON):
{
  "title": "PR Title",
  "summary": "PR Summary in Markdown"
}
`;
  const response = await openai.chat.completions.create({
    model: "gpt-4o",
    messages: [{ role: "user", content: prompt }],
    response_format: { type: "json_object" }
  });
  const content = response.choices[0].message.content;
  if (!content) {
    throw new Error("Failed to generate PR details");
  }
  return JSON.parse(content);
}

// src/commands/pr.ts
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
var prCommand = new import_commander2.Command("pr").description("Pull Request management");
prCommand.command("create").description("Create a Pull Request").argument("[ticket-number]", "Ticket number (defaults to current branch)").action(async (ticketNumber) => {
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
      specContent = await import_promises2.default.readFile(specFile, "utf-8");
    } else {
      console.warn(`Warning: Spec file not found for ticket ${ticket}.`);
    }
    const diff = await getDiff();
    console.log("Generating PR details with OpenAI...");
    let title = `feat: ${ticket}`;
    let summary = "";
    try {
      const details = await generatePrDetails(diff, specContent);
      title = details.title;
      summary = details.summary;
    } catch (error) {
      console.error("Failed to generate PR details with OpenAI, falling back to defaults.");
      console.error(error);
    }
    const prBody = `${summary}

## User original prompt

${specContent}`;
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
var pingCommand = new import_commander3.Command("ping").description("Ping the CLI to check if it is alive").action(() => {
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
