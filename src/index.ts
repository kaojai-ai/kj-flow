#!/usr/bin/env node
import { Command } from 'commander';
import dotenv from 'dotenv';
import { version } from '../package.json';
import { specCommand } from './commands/spec';
import { prCommand } from './commands/pr';
import { pingCommand } from './commands/ping';

dotenv.config();

const program = new Command();

program
    .name('kj')
    .description('Kaojai Flow CLI')
    .version(version);

program.addCommand(specCommand);
program.addCommand(prCommand);
program.addCommand(pingCommand);

program.parse(process.argv);
