import { Command } from 'commander';
import { getCurrentBranch, isGitHubRemote } from '../utils/git';
import { execa } from 'execa';
import fs from 'fs/promises';
import path from 'path';

async function findSpecFile(ticketNumber: string): Promise<string | null> {
    const specsDir = path.join(process.cwd(), 'specs');
    try {
        const years = await fs.readdir(specsDir);
        for (const year of years) {
            const yearPath = path.join(specsDir, year);
            const yearStat = await fs.stat(yearPath);
            if (!yearStat.isDirectory()) continue;

            const months = await fs.readdir(yearPath);
            for (const month of months) {
                const monthPath = path.join(yearPath, month);
                const monthStat = await fs.stat(monthPath);
                if (!monthStat.isDirectory()) continue;

                const tickets = await fs.readdir(monthPath);
                if (tickets.includes(ticketNumber)) {
                    const ticketPath = path.join(monthPath, ticketNumber);
                    const files = await fs.readdir(ticketPath);
                    const mdFile = files.find(f => f.endsWith('.md'));
                    if (mdFile) {
                        return path.join(ticketPath, mdFile);
                    }
                }
            }
        }
    } catch (e) {
        // Ignore errors (e.g. specs dir not found)
    }
    return null;
}

async function getArtifactsMetadata(ticketNumber: string): Promise<{ pr_title: string; pr_summary: string } | null> {
    const specsDir = path.join(process.cwd(), 'specs');
    try {
        const years = await fs.readdir(specsDir);
        for (const year of years) {
            const yearPath = path.join(specsDir, year);
            const yearStat = await fs.stat(yearPath);
            if (!yearStat.isDirectory()) continue;

            const months = await fs.readdir(yearPath);
            for (const month of months) {
                const monthPath = path.join(yearPath, month);
                const monthStat = await fs.stat(monthPath);
                if (!monthStat.isDirectory()) continue;

                const tickets = await fs.readdir(monthPath);
                if (tickets.includes(ticketNumber)) {
                    const metadataPath = path.join(monthPath, ticketNumber, 'artifacts', 'metadata.json');
                    try {
                        const content = await fs.readFile(metadataPath, 'utf-8');
                        return JSON.parse(content);
                    } catch (e) {
                        return null;
                    }
                }
            }
        }
    } catch (e) {
        // Ignore
    }
    return null;
}

export const prCommand = new Command('pr')
    .description('Pull Request management');

prCommand.command('create')
    .description('Create a Pull Request')
    .argument('[ticket-number]', 'Ticket number (defaults to current branch)')
    .option('--dry-run', 'Dry run mode')
    .action(async (ticketNumber, options) => {
        try {
            const currentBranch = await getCurrentBranch();
            const ticket = ticketNumber || currentBranch;

            if (!await isGitHubRemote()) {
                console.error('Error: Not a GitHub repository or remote is not configured.');
                process.exit(1);
            }

            console.log(`Creating PR for ticket: ${ticket}...`);

            // Find spec file
            const specFile = await findSpecFile(ticket);
            let specContent = '';
            if (specFile) {
                console.log(`Found spec file: ${specFile}`);
                specContent = await fs.readFile(specFile, 'utf-8');
            } else {
                console.warn(`Warning: Spec file not found for ticket ${ticket}.`);
            }

            // Get metadata
            const metadata = await getArtifactsMetadata(ticket);
            if (!metadata) {
                console.error(`Error: metadata.json not found for ticket ${ticket}. Please ensure specs/YYYY/MM/${ticket}/artifacts/metadata.json exists.`);
                process.exit(1);
            }

            const { pr_title, pr_summary } = metadata;

            // Generate title and summary
            const title = pr_title || `feat: ${ticket}`;

            // Append spec content to summary
            const prBody = `# AI Summary
${pr_summary}

# User Prompt
${specContent}

# Artifacts
- [implementation_plan.md](specs/2025/12/${ticket}/artifacts/implementation_plan.md)
- [walkthrough.md](specs/2025/12/${ticket}/artifacts/walkthrough.md)`;

            if (options.dryRun) {
                console.log('--- Dry Run ---');
                console.log(`Title: ${title}`);
                console.log(`Body:\n${prBody}`);
                console.log(`Head: ${currentBranch}`);
                console.log('Command: gh pr create --title ... --body ... --head ...');
                return;
            }

            // Create PR using gh cli
            console.log('Running gh pr create...');
            await execa('gh', ['pr', 'create', '--title', title, '--body', prBody, '--head', currentBranch], { stdio: 'inherit' });

        } catch (error) {
            console.error('Error creating PR:', error);
            process.exit(1);
        }
    });

prCommand.command('merge')
    .description('Merge a Pull Request')
    .argument('[ticket-number]', 'Ticket number (defaults to current branch)')
    .action(async (ticketNumber) => {
        try {
            // ticketNumber is unused in gh pr merge if we are on the branch,
            // but if provided we might need to checkout or specify it?
            // Spec says: "ticket-number is optional, default to current branch name... just run gh pr merge"
            // gh pr merge defaults to current branch.

            console.log('Running gh pr merge...');
            await execa('gh', ['pr', 'merge', '--auto', '--merge'], { stdio: 'inherit' });
        } catch (error) {
            console.error('Error merging PR:', error);
            process.exit(1);
        }
    });
