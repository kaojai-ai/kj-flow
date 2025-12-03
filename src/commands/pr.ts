import { Command } from 'commander';
import { getCurrentBranch, isGitHubRemote, getDiff } from '../utils/git';
import { generatePrDetails } from '../utils/openai';
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

export const prCommand = new Command('pr')
    .description('Pull Request management');

prCommand.command('create')
    .description('Create a Pull Request')
    .argument('[ticket-number]', 'Ticket number (defaults to current branch)')
    .action(async (ticketNumber) => {
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

            // Get diff
            const diff = await getDiff();

            // Generate title and summary
            console.log('Generating PR details with OpenAI...');
            let title = `feat: ${ticket}`;
            let summary = '';

            try {
                const details = await generatePrDetails(diff, specContent);
                title = details.title;
                summary = details.summary;
            } catch (error) {
                console.error('Failed to generate PR details with OpenAI, falling back to defaults.');
                console.error(error);
            }

            // Append spec content to summary
            const prBody = `${summary}\n\n## User original prompt\n\n${specContent}`;

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
