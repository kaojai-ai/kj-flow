import { Command } from 'commander';
import { format } from 'date-fns';
import path from 'path';
import fs from 'fs/promises';
import { execa } from 'execa';

export const specCommand = new Command('spec')
    .description('Create a new spec file')
    .argument('<ticket-number>', 'Ticket number')
    .argument('[summary]', 'Summary of the spec')
    .action(async (ticketNumber, summary) => {
        const now = new Date();
        const year = format(now, 'yyyy');
        const month = format(now, 'MM');

        const fileName = summary || ticketNumber;
        const relativePath = path.join('specs', year, month, ticketNumber);
        const absolutePath = path.resolve(process.cwd(), relativePath);
        const filePath = path.join(absolutePath, `${fileName}.md`);

        try {
            await fs.mkdir(absolutePath, { recursive: true });

            // Check if file exists, if not create it
            try {
                await fs.access(filePath);
                console.log(`File already exists: ${filePath}`);
            } catch {
                await fs.writeFile(filePath, `# ${ticketNumber}: ${summary || ''}\n\n`);
                console.log(`Created spec file: ${filePath}`);
            }

            // Open in IDE
            // Default to 'antigravity', but can be configured (env var for now?)
            const ide = process.env.KJ_IDE || 'antigravity';
            console.log(`Opening in ${ide}...`);

            // We use execa to run the command.
            // We don't await it strictly if we want to detach, but usually editors return immediately or we want to wait.
            // For 'antigravity', we assume it might be an alias or binary.
            // If it fails, we might want to fallback or just log error.
            try {
                await execa(ide, [filePath], { stdio: 'inherit' });
            } catch (error) {
                console.error(`Failed to open with ${ide}. Please check if it is installed or set KJ_IDE env var.`);
                console.error(error);
            }

        } catch (error) {
            console.error('Error creating spec:', error);
            process.exit(1);
        }
    });
