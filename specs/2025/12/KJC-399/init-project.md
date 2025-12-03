Build a npm binary cli that will help developer do these tasks

`kj spec [ticket-number] [summary]`
- this will create [summary].md find under new folder under specs folder of this woking directory under /YYYY/MM/[ticket-number]
- and open this file in user config ide (Default to antigravity)
- summary is optional, default to ticket-number

`kj pr create [ticket-numnber]`
- ticket-number is optional, default to current branch name
- this command will read git remote, if it's github, it will use `gh` cli to do following
- create a pull request
- call OpenAI to summarize the changes, and get PR title, and summary
- add section # User original prompt, and attached the markdown file of this spec into PR

`kj pr merge [ticket-number]`
- ticket-number is optional, default to current branch name
- just run `gh pr merge`
