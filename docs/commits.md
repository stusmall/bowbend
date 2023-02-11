This project requires a contributions to meet certain standards with commits. Following these standards helps make code
easier to review, release notes easier to generate, and makes software archaeology easier.

* Changes should be contained individual, revertable commits
* During the code review process requested changes should be added as new commits but squashed before merging. This
  helps the reviewer focus on just the changes, but keeps the commit history clean.
* Each commit should have a clear commit message that stands on its describing the change, why, and how. This is valued
  over PR description. When making PRs it is best to just write a full detailed commit message and let the PR be
  generated from that. The commit history is the record of truth for changes, not GitHub.
* PRs should be rebased off main before merging to keep merge commits out.
* All commits should be categorized using conventional commits.

## Conventional Commits

This project is using [conventional commits](https://www.conventionalcommits.org/en/v1.0.0/) to help automatically build
change logs for each release. Until we hit a certain level of maturity on this project, all changes are allowed to be
breaking so ! will not be used yet.

The types used are:

- feat - This is used for all new features.
- fix - Bug fixes.
- docs - This is for expansion of documentation, either doc strings or markdown readmes
- ci - Improvements on CI, automated releases etc
- chore - This includes changes that don't fall into any of the above categories. Things like bumping deps is a good
  example

The scopes used are:

- core - Changes for the core engine that could affect all SDKs and all scan types
- python - Changes that only impact the python SDK