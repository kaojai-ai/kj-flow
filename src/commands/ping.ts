import { Command } from 'commander';

export const pingCommand = new Command('ping')
    .description('Ping the CLI to check if it is working')
    .action(() => {
        console.log('pong');
    });
