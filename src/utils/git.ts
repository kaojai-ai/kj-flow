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

export async function getRepoInfo(): Promise<{ owner: string; repo: string } | null> {
    const remoteUrl = await getRemoteUrl();
    // Matches git@github.com:owner/repo.git or https://github.com/owner/repo.git
    const match = remoteUrl.match(/github\.com[:/]([^/]+)\/([^/.]+)(?:\.git)?/);
    if (match) {
        return { owner: match[1], repo: match[2] };
    }
    return null;
}

export async function createBranch(branchName: string): Promise<void> {
    await execa('git', ['branch', branchName]);
}

export async function checkoutBranch(branchName: string): Promise<void> {
    await execa('git', ['checkout', branchName]);
}
