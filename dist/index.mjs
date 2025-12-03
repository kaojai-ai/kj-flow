#!/usr/bin/env node

// src/index.ts
import { Command as Command4 } from "commander";
import dotenv from "dotenv";

// package.json
var version = "1.0.0";

// src/commands/spec.ts
import { Command } from "commander";
import { format } from "date-fns";
import path from "path";
import fs from "fs/promises";
import { execa } from "execa";
var specCommand = new Command("spec").description("Create a new spec file").argument("<ticket-number>", "Ticket number").argument("[summary]", "Summary of the spec").action(async (ticketNumber, summary) => {
  const now = /* @__PURE__ */ new Date();
  const year = format(now, "yyyy");
  const month = format(now, "MM");
  const fileName = summary || ticketNumber;
  const relativePath = path.join("specs", year, month, ticketNumber);
  const absolutePath = path.resolve(process.cwd(), relativePath);
  const filePath = path.join(absolutePath, `${fileName}.md`);
  try {
    await fs.mkdir(absolutePath, { recursive: true });
    try {
      await fs.access(filePath);
      console.log(`File already exists: ${filePath}`);
    } catch {
      await fs.writeFile(filePath, `# ${ticketNumber}: ${summary || ""}

`);
      console.log(`Created spec file: ${filePath}`);
    }
    const ide = process.env.KJ_IDE || "antigravity";
    console.log(`Opening in ${ide}...`);
    try {
      await execa(ide, [filePath], { stdio: "inherit" });
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
import { Command as Command2 } from "commander";

// src/utils/git.ts
import { execa as execa2 } from "execa";
async function getCurrentBranch() {
  const { stdout } = await execa2("git", ["branch", "--show-current"]);
  return stdout.trim();
}
async function getRemoteUrl() {
  try {
    const { stdout } = await execa2("git", ["remote", "get-url", "origin"]);
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
    const { stdout } = await execa2("git", ["diff", baseBranch]);
    return stdout;
  } catch {
    return "";
  }
}

// src/utils/openai.ts
import OpenAI from "openai";
async function generatePrDetails(diff, specContent) {
  const apiKey = process.env.OPENAI_API_KEY;
  if (!apiKey) {
    throw new Error("OPENAI_API_KEY is not set");
  }
  const openai = new OpenAI({ apiKey });
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
import { execa as execa3 } from "execa";
import fs2 from "fs/promises";
import path2 from "path";
async function findSpecFile(ticketNumber) {
  const specsDir = path2.join(process.cwd(), "specs");
  try {
    const years = await fs2.readdir(specsDir);
    for (const year of years) {
      const yearPath = path2.join(specsDir, year);
      const yearStat = await fs2.stat(yearPath);
      if (!yearStat.isDirectory()) continue;
      const months = await fs2.readdir(yearPath);
      for (const month of months) {
        const monthPath = path2.join(yearPath, month);
        const monthStat = await fs2.stat(monthPath);
        if (!monthStat.isDirectory()) continue;
        const tickets = await fs2.readdir(monthPath);
        if (tickets.includes(ticketNumber)) {
          const ticketPath = path2.join(monthPath, ticketNumber);
          const files = await fs2.readdir(ticketPath);
          const mdFile = files.find((f) => f.endsWith(".md"));
          if (mdFile) {
            return path2.join(ticketPath, mdFile);
          }
        }
      }
    }
  } catch (e) {
  }
  return null;
}
var prCommand = new Command2("pr").description("Pull Request management");
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
      specContent = await fs2.readFile(specFile, "utf-8");
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
    await execa3("gh", ["pr", "create", "--title", title, "--body", prBody, "--head", currentBranch], { stdio: "inherit" });
  } catch (error) {
    console.error("Error creating PR:", error);
    process.exit(1);
  }
});
prCommand.command("merge").description("Merge a Pull Request").argument("[ticket-number]", "Ticket number (defaults to current branch)").action(async (ticketNumber) => {
  try {
    console.log("Running gh pr merge...");
    await execa3("gh", ["pr", "merge", "--auto", "--merge"], { stdio: "inherit" });
  } catch (error) {
    console.error("Error merging PR:", error);
    process.exit(1);
  }
});

// src/commands/ping.ts
import { Command as Command3 } from "commander";
var pingCommand = new Command3("ping").description("Ping the CLI to check if it is working").action(() => {
  console.log("pong");
});

// src/index.ts
dotenv.config();
var program = new Command4();
program.name("kj").description("Kaojai Flow CLI").version(version);
program.addCommand(specCommand);
program.addCommand(prCommand);
program.addCommand(pingCommand);
program.parse(process.argv);
