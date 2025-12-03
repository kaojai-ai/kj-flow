import { execa } from 'execa';

export async function getCurrentBranch(): Promise<string> {
    const { stdout } = await execa('git', ['branch', '--show-current']);
    return stdout.trim();
}

export async function getRemoteUrl(): Promise<string> {
    try {
        const { stdout } = await execa('git', ['remote', 'get-url', 'origin']);
        return stdout.trim();
    } catch {
        return '';
    }
}

export async function isGitHubRemote(): Promise<boolean> {
    const remoteUrl = await getRemoteUrl();
    return remoteUrl.includes('github.com');
}

export async function getDiff(baseBranch: string = 'main'): Promise<string> {
    try {
        const { stdout } = await execa('git', ['diff', baseBranch]);
        return stdout;
    } catch {
        return '';
    }
}
