#!/usr/bin/env node

// src/index.ts
import { Command as Command4 } from "commander";
import dotenv from "dotenv";

// package.json
var version = "1.0.3";

// src/commands/spec.ts
import { Command } from "commander";
import { format } from "date-fns";
import path from "path";
import fs from "fs/promises";
import { execa as execa2 } from "execa";

// src/utils/git.ts
import { execa } from "execa";
async function getCurrentBranch() {
  const { stdout } = await execa("git", ["branch", "--show-current"]);
  return stdout.trim();
}
async function getRemoteUrl() {
  try {
    const { stdout } = await execa("git", ["remote", "get-url", "origin"]);
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
  await execa("git", ["branch", branchName]);
}
async function checkoutBranch(branchName) {
  await execa("git", ["checkout", branchName]);
}

// src/commands/spec.ts
import readline from "readline";
function askQuestion(query) {
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
  });
  return new Promise((resolve) => rl.question(query, (ans) => {
    rl.close();
    resolve(ans);
  }));
}
var specCommand = new Command("spec").description("Create a new spec file").argument("<ticket-number>", "Ticket number").argument("[summary]", "Summary of the spec").action(async (ticketNumber, summary) => {
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
      await fs.writeFile(filePath, `---
ticket_number: ${ticketNumber}
---

${summary || ""}
`);
      console.log(`Created spec file: ${filePath}`);
    }
    const ide = process.env.KJ_IDE || "antigravity";
    console.log(`Opening in ${ide}...`);
    try {
      await execa2(ide, [filePath], { stdio: "inherit" });
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
async function getArtifactsMetadata(ticketNumber) {
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
          const metadataPath = path2.join(monthPath, ticketNumber, "artifacts", "metadata.json");
          try {
            const content = await fs2.readFile(metadataPath, "utf-8");
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
var prCommand = new Command2("pr").description("Pull Request management");
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
      const content = await fs2.readFile(specFile, "utf-8");
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
